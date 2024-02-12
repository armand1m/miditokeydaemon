use config::{Config, File, FileFormat};
use enigo::{Enigo, KeyboardControllable};
use log;
use midir::{MidiInput, MidiInputPort};
use std::thread;
use std::time::Duration;

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Settings {
    pub device_port_name: String,
    pub midi_mapping: Vec<MidiMap>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct MidiMap {
    pub midi_id: u8,
    pub note: u8,
    pub keymap: Option<String>,
    pub velocity: Option<u8>,
    pub command: Option<String>,
    pub options: Option<MidiMapOptions>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct MidiMapOptions {
    pub velocity: Option<MidiMapVelocityOptions>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct MidiMapVelocityOptions {
    pub debounce: Option<bool>,
    pub scale: Option<VelocityScale>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct VelocityScale {
    pub min: u8,
    pub max: u8,
}

fn main() {
    env_logger::init();

    let config_file_path = shellexpand::tilde("~/.miditokeydaemonrc");

    let config = Config::builder()
        .add_source(File::new(&config_file_path, FileFormat::Json))
        .build()
        .expect("Failed to read configuration file");

    let settings = config
        .try_deserialize::<Settings>()
        .expect("Failed to deserialize daemon settings.");

    log::debug!("Settings: {:#?}", settings);

    let midi_input = MidiInput::new("miditokeydaemon").expect("Failed to read MIDI input.");

    let device_ports: Vec<MidiInputPort> = midi_input
        .ports()
        .into_iter()
        .filter_map(|port| {
            let port_name = midi_input
                .port_name(&port)
                .expect("Failed to read port name.");

            log::debug!("Port found: {:?}", port_name);

            if port_name.contains(&settings.device_port_name) {
                return Some(port);
            }

            None
        })
        .collect();

    let port = device_ports
        .get(0)
        .expect("No MIDI ports available for the specified 'device_port_name' property in the configuration.");

    let port_name = midi_input.port_name(port).unwrap();

    log::debug!("Selected MIDI Port: {}", port_name);

    let _connection = midi_input
        .connect(
            port.into(),
            port_name.as_str(),
            move |timestamp, message, settings| {
                log::debug!("[{}] Received MIDI message: {:?}", timestamp, message);
                let _ = process_midi_message(message, settings);
            },
            settings,
        )
        .expect("Failed to connect to MIDI input port");

    log::debug!("Daemon is initialized.");

    loop {
        thread::sleep(Duration::from_millis(100));
    }
}

fn match_velocity(velocity: Option<u8>, mapping: &MidiMap) -> bool {
    if let Some(mapping_velocity) = mapping.velocity {
        if let Some(actual_velocity) = velocity {
            return actual_velocity == mapping_velocity;
        }
    }
    true
}

fn get_computed_velocity(velocity: Option<u8>, mapping: &MidiMap) -> Option<u8> {
    let velocity_scale = mapping.options.clone()?.velocity?.scale;

    match velocity_scale {
        Some(scale) => Some(scale_value(velocity?, scale.min, scale.max)),
        None => velocity,
    }
}

fn process_midi_message(message: &[u8], settings: &Settings) -> Result<(), anyhow::Error> {
    let midi_id = message[0];
    let note = message[1];
    let device_velocity = message.get(2).cloned();

    let mut enigo = Enigo::new();

    for mapping in &settings.midi_mapping {
        let match_command = midi_id == mapping.midi_id;
        let match_note = note == mapping.note;
        let matches_velocity = match_velocity(device_velocity, mapping);
        let mapping_match = match_command && match_note && matches_velocity;

        if !mapping_match {
            continue;
        }

        log::debug!(
            "Found a midi_id '{}', note '{}' and velocity match.",
            midi_id,
            note
        );

        if let Some(keymap) = &mapping.keymap {
            log::debug!("Parsing key sequence: {}", keymap);
            enigo.key_sequence_parse(keymap.as_str());
        }

        if let Some(command) = &mapping.command {
            // TODO: add debouncing logic
            let command_str = command.as_str();

            if command_str == "" {
                continue;
            }

            let err_message = format!("'{}' command failed to start", command_str);
            let mut process = std::process::Command::new("sh");

            let computed_velocity = get_computed_velocity(device_velocity, mapping);
            if let Some(velocity_value) = computed_velocity {
                process.env("MIDI_VELOCITY", format!("{}", velocity_value));
            }

            log::debug!("Running command: sh -c {}", command_str);

            process
                .arg("-c")
                .arg(command_str)
                .spawn()
                .expect(&err_message);
        }

        // TODO: add mouse event capture
    }

    Ok(())
}

fn scale_value(input: u8, min: u8, max: u8) -> u8 {
    let range = max as f32 - min as f32;
    let scale_factor = range / 127.0;
    let output = min as f32 + ((input as f32) * scale_factor);
    output.round() as u8
}
