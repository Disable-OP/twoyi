#!/bin/bash
# Quick-install APK on emulator before OOM kills QEMU
# This script polls for ADB connection and installs the APK immediately

export PATH=/home/z/my-project/.android-sdk/platform-tools:/home/z/my-project/.android-sdk/emulator:$PATH
export ANDROID_HOME=/home/z/my-project/.android-sdk
export LD_PRELOAD=/home/z/my-project/scripts/fake_statvfs.so

APK=/home/z/my-project/download/twoyi_3.5.5-08052327-release.apk
RESULT_FILE=/tmp/quick_install_result.txt

# Kill any existing emulator
pkill -9 emulator 2>/dev/null
pkill -9 qemu 2>/dev/null
pkill -9 crashpad 2>/dev/null
sleep 2

# Start emulator
echo "Starting emulator..." > $RESULT_FILE
nohup emulator -avd twoyi28 \
  -no-window -no-audio -no-snapshot -no-boot-anim \
  -gpu swiftshader_indirect -accel off -memory 768 \
  -no-cache -show-kernel -selinux permissive \
  > /tmp/emu_quick.log 2>&1 &

EMU_PID=$!
echo "Emulator PID: $EMU_PID" >> $RESULT_FILE

# Poll for ADB connection — install APK the moment it connects
echo "Polling for ADB..." >> $RESULT_FILE
for i in $(seq 1 60); do
    sleep 5
    state=$(adb get-state 2>&1)
    echo "  ${i}x5s: state=$state" >> $RESULT_FILE

    if [ "$state" = "device" ]; then
        echo "DEVICE CONNECTED! Installing APK..." >> $RESULT_FILE
        adb install -r "$APK" >> $RESULT_FILE 2>&1
        echo "Install exit code: $?" >> $RESULT_FILE

        echo "Checking installed packages..." >> $RESULT_FILE
        adb shell pm list packages | grep twoyi >> $RESULT_FILE 2>&1

        echo "Launching SettingsActivity..." >> $RESULT_FILE
        adb shell am start -n io.twoyi/.ui.SettingsActivity >> $RESULT_FILE 2>&1

        sleep 10
        echo "Taking logcat..." >> $RESULT_FILE
        adb logcat -d -t 100 >> $RESULT_FILE 2>&1

        echo "Taking screenshot..." >> $RESULT_FILE
        adb shell screencap /sdcard/screen.png >> $RESULT_FILE 2>&1
        adb pull /sdcard/screen.png /home/z/my-project/download/emulator_booted.png >> $RESULT_FILE 2>&1

        echo "DONE!" >> $RESULT_FILE
        break
    fi

    # Check if emulator is still alive
    if ! kill -0 $EMU_PID 2>/dev/null; then
        echo "Emulator died at ${i}x5s" >> $RESULT_FILE
        break
    fi
done

echo "Script finished." >> $RESULT_FILE
