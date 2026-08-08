#!/usr/bin/env python3
"""
Add the 22 missing string resources to all 4 locale strings.xml files.

The Java code references these strings (e.g. R.string.error_generic,
R.string.profile_switch_failed, R.string.rom_import_failed) but they
were never actually committed to strings.xml despite MEMORY.md
claiming they were added in round 32. As a result every CI build
since at least round 60 has been failing at compileDebugJavaWithJavac
with "error: cannot find symbol" — but this was masked by the earlier
nttld/setup-ndk@v2 failure (CI never got far enough to compile Java).

This script is idempotent: it checks whether each string already
exists before inserting, so re-running it is safe.
"""

import sys
from pathlib import Path

# 22 missing strings, in 4 locales.
# Format: (key, EN, zh-CN, zh-TW, ja)
# Note: zh-TW uses traditional characters; ja is Japanese.
# Placeholders (%1$s, %1$d, %2$d) MUST be preserved verbatim across
# all locales — Android's formatter replaces them positionally.
MISSING_STRINGS = [
    # ---- generic error / utility strings ----
    ("error_generic",
     "Error",
     "错误",
     "錯誤",
     "エラー"),
    ("error_selecting_file",
     "Error selecting file",
     "选择文件失败",
     "選擇檔案失敗",
     "ファイルの選択に失敗しました"),
    ("error_sharing_log",
     "Error sharing log",
     "分享日志失败",
     "分享日誌失敗",
     "ログの共有に失敗しました"),
    # ---- WeChat-specific ----
    ("wechat_not_installed",
     "WeChat is not installed.",
     "微信未安装。",
     "微信未安裝。",
     "WeChatがインストールされていません。"),
    # ---- settings validation ----
    ("settings_invalid_number",
     "Invalid number",
     "无效的数字",
     "無效的數字",
     "無効な数値"),
    ("settings_width_range_error",
     "Width must be between %1$d and %2$d",
     "宽度必须在 %1$d 和 %2$d 之间",
     "寬度必須在 %1$d 和 %2$d 之間",
     "幅は %1$d から %2$d の間でなければなりません"),
    ("settings_height_range_error",
     "Height must be between %1$d and %2$d",
     "高度必须在 %1$d 和 %2$d 之间",
     "高度必須在 %1$d 和 %2$d 之間",
     "高さは %1$d から %2$d の間でなければなりません"),
    ("settings_dpi_range_error",
     "DPI must be between %1$d and %2$d",
     "DPI 必须在 %1$d 和 %2$d 之间",
     "DPI 必須在 %1$d 和 %2$d 之間",
     "DPIは %1$d から %2$d の間でなければなりません"),
    # ---- ROM import flow ----
    ("rom_imported_successfully",
     "ROM imported successfully",
     "ROM 导入成功",
     "ROM 匯入成功",
     "ROMのインポートに成功しました"),
    ("rom_import_failed",
     "Failed to import ROM",
     "ROM 导入失败",
     "ROM 匯入失敗",
     "ROMのインポートに失敗しました"),
    ("rom_import_error",
     "Error importing ROM: %1$s",
     "导入 ROM 出错：%1$s",
     "匯入 ROM 出錯：%1$s",
     "ROMのインポート中にエラーが発生しました: %1$s"),
    # ---- profile manager flow ----
    ("profile_switch_failed",
     "Failed to switch profile",
     "切换配置文件失败",
     "切換設定檔失敗",
     "プロファイルの切り替えに失敗しました"),
    ("profile_renamed_to",
     "Profile renamed to %1$s",
     "配置文件已重命名为 %1$s",
     "設定檔已重新命名為 %1$s",
     "プロファイル名を %1$s に変更しました"),
    ("profile_rename_failed",
     "Failed to rename profile",
     "重命名配置文件失败",
     "重新命名設定檔失敗",
     "プロファイル名の変更に失敗しました"),
    ("profile_copied_to",
     "Profile copied to %1$s",
     "配置文件已复制为 %1$s",
     "設定檔已複製為 %1$s",
     "プロファイルを %1$s にコピーしました"),
    ("profile_copy_failed",
     "Failed to copy profile",
     "复制配置文件失败",
     "複製設定檔失敗",
     "プロファイルのコピーに失敗しました"),
    ("profile_copy_error",
     "Error copying profile: %1$s",
     "复制配置文件出错：%1$s",
     "複製設定檔出錯：%1$s",
     "プロファイルのコピー中にエラーが発生しました: %1$s"),
    ("profile_deleted",
     "Profile %1$s deleted",
     "配置文件 %1$s 已删除",
     "設定檔 %1$s 已刪除",
     "プロファイル %1$s を削除しました"),
    ("profile_delete_failed",
     "Failed to delete profile",
     "删除配置文件失败",
     "刪除設定檔失敗",
     "プロファイルの削除に失敗しました"),
    ("profile_picker_error",
     "Error picking profile",
     "选择配置文件出错",
     "選擇設定檔出錯",
     "プロファイルの選択中にエラーが発生しました"),
    ("profile_created",
     "Profile %1$s created",
     "配置文件 %1$s 已创建",
     "設定檔 %1$s 已建立",
     "プロファイル %1$s を作成しました"),
    ("profile_create_failed",
     "Failed to create profile",
     "创建配置文件失败",
     "建立設定檔失敗",
     "プロファイルの作成に失敗しました"),
]

LOCALES = [
    # (file_suffix, column_index_in_values_list)
    # values_list is the post-unpack list: [en, zh-CN, zh-TW, ja]
    # so 0=EN, 1=zh-CN, 2=zh-TW, 3=ja.
    ("",          0),  # values/strings.xml        — English (default)
    ("-zh-rCN",   1),  # values-zh-rCN/strings.xml — Simplified Chinese
    ("-zh-rTW",   2),  # values-zh-rTW/strings.xml — Traditional Chinese
    ("-ja",       3),  # values-ja/strings.xml     — Japanese
]

REPO_ROOT = Path("/home/z/my-project/repos/twoyi")
RES_DIR = REPO_ROOT / "app/src/main/res"


def find_string(xml_text: str, key: str) -> bool:
    """Return True if `key` is already defined as a <string> in xml_text."""
    needle = f'name="{key}"'
    return needle in xml_text


def insert_strings(xml_path: Path, strings_to_add: list[tuple[str, str]]) -> int:
    """
    Insert the given (key, value) pairs into the <resources> block,
    immediately before the closing </resources> tag. Skips any key
    that already exists (idempotent). Returns the number of strings
    actually inserted.
    """
    text = xml_path.read_text(encoding="utf-8")
    inserted = 0
    additions = []
    for key, value in strings_to_add:
        if find_string(text, key):
            print(f"  [skip] {key} already exists in {xml_path.name}")
            continue
        # Escape XML special chars in the value. We do NOT escape
        # apostrophes/quotes here because Android string resources
        # use plain UTF-8 text; the existing strings in the file
        # follow this convention (e.g. "Don\'t show again" is a
        # manual style choice, not a requirement).
        safe = (
            value
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
        )
        additions.append(f'    <string name="{key}">{safe}</string>')
        inserted += 1

    if not additions:
        return 0

    # Insert before the closing </resources> tag. We match the LAST
    # occurrence (there should only be one) and preserve any trailing
    # whitespace/newline before it.
    closing = "</resources>"
    idx = text.rfind(closing)
    if idx == -1:
        print(f"  [error] no </resources> tag found in {xml_path}")
        return 0

    # Build the new text: text[:idx] + additions + closing + text[idx+len(closing):]
    # Preserve a single newline between the last addition and </resources>.
    block = "\n".join(additions) + "\n"
    new_text = text[:idx] + block + text[idx:]
    xml_path.write_text(new_text, encoding="utf-8")
    return inserted


def main() -> int:
    total_inserted = 0
    for suffix, col in LOCALES:
        xml_path = RES_DIR / f"values{suffix}" / "strings.xml"
        if not xml_path.exists():
            print(f"[error] missing file: {xml_path}")
            return 1
        print(f"\n=== {xml_path.relative_to(REPO_ROOT)} ===")
        strings_for_locale = [(key, values[col]) for key, *values in MISSING_STRINGS]
        inserted = insert_strings(xml_path, strings_for_locale)
        print(f"  inserted {inserted} new strings")
        total_inserted += inserted
    print(f"\n=== TOTAL: {total_inserted} strings inserted across 4 locales ===")
    return 0


if __name__ == "__main__":
    sys.exit(main())
