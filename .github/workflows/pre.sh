#!/bin/sh
if [ "$(uname)" == "Linux" ]; then
    # Check if it's Ubuntu
    if [ -f /etc/os-release ]; then
        source /etc/os-release
        if [ "$ID" == "ubuntu" ]; then
            sudo apt-get update
            sudo apt-get install -y librust-alsa-sys-dev libfl-dev libxdo-dev
            echo "Package installed successfully on Ubuntu."
        else
            echo "This script is intended for Ubuntu, but the detected Linux distribution is $ID."
        fi
    else
        echo "Unable to determine the Linux distribution."
    fi
else
    echo "This script is intended for Linux systems, but the detected OS is not Linux."
fi