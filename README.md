# miditokeydaemon

`miditokeydaemon` is a configurable background process that allows one to map midi messages into keymap or shell commands.

The inspiration to build this was because of the lack of options for Mac OS users. Most options out there are either way too old, clunky or at least paid.
This is a simple way out of that. 

_(I mean, it is free. I already put the work for you, just use it!)_

## Features

- [x] Support JSON configuration on `~/.miditokeydaemonrc`
- [x] Allow for multiple keymaps based on velocity values and CC notes
- [x] Allow for shell commands to be used for specific MIDI commands
    - [x] Debouncing for velocity-based commands can be set using the `options.velocity.debounce` property in a midi mapping.
    - [x] Set env vars for variable MIDI properties:
        - [x] Velocity: Available as the `$MIDI_VELOCITY` env var
- [x] Allow for rescaling velocity levels to a specific range.
    - Velocity ranges are usually from 0 to 127, and one might want to increase or decrease this range. The configuration file supports setting a range for velocity messages being used in commands.
- [ ] Support for mouse events
- [ ] Specify custom configuration file path

## How to install

### macOS

For now you'll have to clone and build this repository locally with `cargo build --release`.

In the future, you'll be able to download the built binary for your platform from the Github Releases archive and install it into `/usr/local/bin/miditokeydaemon`.

Run the following script to register the `miditokeydaemon` as a background process:

```sh
cat > ~/Library/LaunchAgents/com.armand1m.miditokeydaemon.plist <<EOL
 <?xml version="1.0" encoding="UTF-8"?>
 <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
 <plist version="1.0">
     <dict>
         <key>Label</key>
         <string>miditokeydaemon</string>
         <key>ProgramArguments</key>
         <array>
             <string>/usr/local/bin/miditokeydaemon</string>
         </array>
         <key>EnvironmentVariables</key>
         <dict>
             <key>RUST_LOG</key>
             <string>debug</string>
         </dict>
         <key>RunAtLoad</key>
         <true/>
         <key>KeepAlive</key>
         <dict>
             <key>SuccessfulExit</key>
             <false/>
             <key>Crashed</key>
             <true/>
         </dict>
         <key>StandardOutPath</key>
         <string>/tmp/miditokeydaemon.out.log</string>
         <key>StandardErrorPath</key>
         <string>/tmp/miditokeydaemon.err.log</string>
         <key>ProcessType</key>
         <string>Interactive</string>
         <key>Nice</key>
         <integer>-20</integer>
     </dict>
 </plist>
EOL
launchctl load ~/Library/LaunchAgents/com.armand1m.miditokeydaemon.plist
```

You should now have the daemon loaded and stopped:

```sh
launchctl list | grep miditokeydaemon
```

Now you should prepare your config at `~/.miditokeydaemonrc` _(read next session then come back)_.

Once you're done with the configuration, start your daemon:

```sh
launchctl start miditokeydaemon
```

You can check if the process is running:

```sh
ps aux | grep miditokeydaemon
```

### Windows and Linux

For now you'll have to clone and build this repository locally with `cargo build --release`.

Prepare the daemon-manager/scheduler of your choice in your OS.

Now you should prepare your config at `~/.miditokeydaemonrc` and start the service on your daemon-manager.

## Example config

You will need to specify a device port name in your configuration for your specific device. This name can be approximate, the code will look for ports containing the string and select the first matching port.

Keymaps are built on top of [Enigo's DSL](https://docs.rs/enigo/latest/src/enigo/dsl.rs.html#1-289), which is unusual but works well cross-platform and supports building macros easily. _(I'll update the docs with the available keys table)_

I strongly recommend running the daemon with `RUST_LOG=debug` to be able to see debug messages with details from the MIDI messages from your device and tweak the configuration accordingly.

Below are some examples in the example config for my FBV Express Mk II. This configuration should be placed on your $HOME directory as `.miditokeydaemonrc`.

```json
{
  "device_port_name": "FBV Express Mk II",
  "midi_mapping": [
    {
      "midi_id": 176,
      "note": 21,
      "keymap": "{+META}xf{-META}"
    },
    {
      "midi_id": 176,
      "note": 22,
      "keymap": "{+CTRL}y{-CTRL}"
    },
    {
      "midi_id": 176,
      "note": 23,
      "velocity": 0,
      "keymap": "gg"
    },
    {
      "midi_id": 176,
      "note": 23,
      "velocity": 127,
      "keymap": "{+SHIFT}gg{-SHIFT}"
    },
    {
      "midi_id": 176,
      "note": 7,
      "command": "osascript -e \"set Volume $MIDI_VELOCITY\"",
      "options": {
        "debounce": 50,
        "velocity": {
          "scale": {
            "min": 0,
            "max": 10
          }
        }
      }
    }
  ] 
}
```

## Developing

To contribute and develop, make sure you have Rust toolchain installed in your dev environment.

```sh
git clone https://github.com/armand1m/miditokeydaemon.git
cd ./miditokeydaemon
RUST_LOG=debug cargo run
```

You should have the daemon running in development mode now.

