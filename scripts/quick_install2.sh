#!/bin/bash
# Quick-install APK on emulator — wait for full boot completion
export PATH=/home/z/my-project/.android-sdk/platform-tools:/home/z/my-project/.android-sdk/emulator:$PATH
export ANDROID_HOME=/home/z/my-project/.android-sdk
export LD_PRELOAD=/home/z/my-project/scripts/fake_statvfs.so

APK=/home/z/my-project/download/twoyi_3.5.5-08052327-release.apk
RESULT_FILE=/tmp/quick_install2.txt

pkill -9 emulator 2>/dev/null; pkill -9 qemu 2>/dev/null; pkill -9 crashpad 2>/dev/null; sleep 2

echo "Starting emulator..." > $RESULT_FILE
nohup emulator -avd twoyi28 \
  -no-window -no-audio -no-snapshot -no-boot-anim \
  -gpu swiftshader_indirect -accel off -memory 768 \
  -no-cache -show-kernel -selinux permissive \
  > /tmp/emu_quick2.log 2>&1 &

EMU_PID=$!
echo "Emulator PID: $EMU_PID" >> $RESULT_FILE

# Wait for ADB device
echo "Waiting for ADB device..." >> $RESULT_FILE
adb wait-for-device 2>>$RESULT_FILE

# Wait for boot_completed=1
echo "Waiting for sys.boot_completed=1..." >> $RESULT_FILE
for i in $(seq 1 60); do
    sleep 5
    boot=$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')
    echo "  ${i}x5s: boot_completed='$boot'" >> $RESULT_FILE
    if [ "$boot" = "1" ]; then
        echo "BOOT COMPLETED!" >> $RESULT_FILE

        # Wait a bit more for package manager to be ready
        echo "Waiting 15s for package manager..." >> $RESULT_FILE
        sleep 15

        echo "Installing APK..." >> $RESULT_FILE
        adb install -r "$APK" >> $RESULT_FILE 2>&1
        echo "Install exit code: $?" >> $RESULT_FILE

        echo "Installed packages:" >> $RESULT_FILE
        adb shell pm list packages | grep twoyi >> $RESULT_FILE 2>&1

        echo "Launching SettingsActivity..." >> $RESULT_FILE
        adb shell am start -n io.twoyi/.ui.SettingsActivity >> $RESULT_FILE 2>&1

        sleep 10
        echo "Twoyi process:" >> $RESULT_FILE
        adb shell ps | grep twoyi >> $RESULT_FILE 2>&1

        echo "Twoyi logcat:" >> $RESULT_FILE
        adb logcat -d | grep -i "twoyi\|CLIENT_EGL\|CORE\|Render2" >> $RESULT_FILE 2>&1

        echo "Taking screenshot..." >> $RESULT_FILE
        adb exec-out screencap -p > /home/z/my-project/download/emulator_booted.png 2>>$RESULT_FILE

        echo "DONE!" >> $RESULT_FILE
        break
    fi

    if ! kill -0 $EMU_PID 2>/dev/null; then
        echo "Emulator died at ${i}x5s" >> $RESULT_FILE
        break
    fi
done

echo "Script finished." >> $RESULT_FILE
