use config::{Config, File, FileFormat};
use enigo::Enigo;
use midir::MidiInput;
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

mod enigo_dsl;

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
    pub mouse: Option<String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct MidiMapOptions {
    pub velocity: Option<MidiMapVelocityOptions>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct MidiMapVelocityOptions {
    pub debounce: Option<u64>,
    pub scale: Option<VelocityScale>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct VelocityScale {
    pub min: u8,
    pub max: u8,
}

fn main() {
    env_logger::init();

    let settings = get_settings();

    log::debug!("Settings: {:#?}", settings);

    let midi_input = MidiInput::new("miditokeydaemon").expect("Failed to read MIDI input.");

    let ports = midi_input.ports();
    let port = ports.iter().find(|port| {
        let port_name = midi_input
            .port_name(port)
            .expect("Failed to read port name.");

        log::debug!("Port found: {:?}", port_name);

        port_name.contains(&settings.device_port_name)
    })
    .expect("No MIDI ports available for the specified 'device_port_name' property in the configuration.");

    let port_name = midi_input.port_name(port).unwrap();

    log::debug!("Selected MIDI Port: {}", port_name);

    let debounce_state: HashMap<String, Instant> = HashMap::new();

    let _connection = midi_input
        .connect(
            port,
            port_name.as_str(),
            move |timestamp, message, (settings, debounce_state)| {
                log::debug!("[{}] Received MIDI message: {:?}", timestamp, message);
                let _ = process_midi_message(message, settings, debounce_state);
            },
            (settings, debounce_state),
        )
        .expect("Failed to connect to MIDI input port");

    log::debug!("Daemon is initialized.");

    loop {
        thread::sleep(Duration::from_millis(100));
    }
}

/// This function reads the settings from the configuration file.
fn get_settings() -> Settings {
    let config_file_path = shellexpand::tilde("~/.miditokeydaemonrc");

    let config = Config::builder()
        .add_source(File::new(&config_file_path, FileFormat::Json))
        .build()
        .expect("Failed to read configuration file");

    config
        .try_deserialize::<Settings>()
        .expect("Failed to deserialize daemon settings.")
}

/// This function checks if the actual velocity matches the mapping velocity.
/// If the mapping velocity is not specified, it returns true.
fn match_velocity(velocity: Option<u8>, mapping: &MidiMap) -> bool {
    mapping.velocity.map_or(true, |mapping_velocity| {
        velocity.map_or(true, |actual_velocity| actual_velocity == mapping_velocity)
    })
}

/// This function computes the velocity based on the mapping options.
/// If the scale is specified, it scales the velocity; otherwise, it returns the original velocity.
fn get_computed_velocity(velocity: Option<u8>, mapping: &MidiMap) -> Option<u8> {
    let velocity_scale = mapping.options.clone()?.velocity?.scale;

    match velocity_scale {
        Some(scale) => Some(scale_value(velocity?, scale.min, scale.max)),
        None => velocity,
    }
}

fn get_debounce_duration(mapping: &MidiMap) -> Duration {
    let duration_value = mapping
        .options
        .clone()
        .and_then(|o| o.velocity)
        .and_then(|v| v.debounce)
        .unwrap_or(200);

    Duration::from_millis(duration_value)
}

/// This function processes a MIDI message based on the settings.
/// It checks each mapping in the settings, and if the MIDI ID, note, and velocity match the mapping,
/// it executes the associated action.
fn process_midi_message(
    message: &[u8],
    settings: &Settings,
    debounce_state: &mut HashMap<String, Instant>,
) -> Result<(), anyhow::Error> {
    let (midi_id, note, device_velocity) = (message[0], message[1], message.get(2).cloned());

    let mut enigo = Enigo::new();

    for mapping in &settings.midi_mapping {
        let mapping_match = midi_id == mapping.midi_id
            && note == mapping.note
            && match_velocity(device_velocity, mapping);

        if !mapping_match {
            continue;
        }

        log::debug!(
            "Found a midi_id '{}', note '{}' and velocity match.",
            midi_id,
            note
        );

        if let Some(keymap) = &mapping.keymap {
            log::debug!("Parsing keymap: {}", keymap);
            if let Err(err) = enigo_dsl::eval(&mut enigo, keymap.as_str()) {
                log::error!("Failed to parse keymap {}", keymap);
                log::error!("{:?}", err);
            }
        }

        if let Some(command) = &mapping.command {
            let command_str = command.as_str();

            if command_str.is_empty() {
                continue;
            }

            let debounce_duration = get_debounce_duration(mapping);

            let state_key = debounce_state
                .clone()
                .into_keys()
                .find(|k| k == command_str);

            if let Some(key) = state_key {
                let last_execution = debounce_state.get_mut(&key).unwrap();
                if last_execution.elapsed() < debounce_duration {
                    log::debug!("Debouncing command: {}", command_str);
                    continue;
                }
            }
            debounce_state.insert(command_str.to_string(), Instant::now());

            let err_message = format!("'{}' command failed to start", command_str);
            let mut process = std::process::Command::new("sh");

            let computed_velocity = get_computed_velocity(device_velocity, mapping);
            if let Some(velocity_value) = computed_velocity {
                log::debug!("Running command: sh -c {}", command_str);
                log::debug!("With $MIDI_VELOCITY being '{}'", velocity_value);
                process.env("MIDI_VELOCITY", format!("{}", velocity_value));
            }

            process
                .arg("-c")
                .arg(command_str)
                .spawn()
                .expect(&err_message);
        }
    }

    Ok(())
}

/// This function scales the input value to the specified range.
fn scale_value(input: u8, min: u8, max: u8) -> u8 {
    let range = max as f32 - min as f32;
    let scale_factor = range / 127.0;
    let output = min as f32 + ((input as f32) * scale_factor);
    output.round() as u8
}
