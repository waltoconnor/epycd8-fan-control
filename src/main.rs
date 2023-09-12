use std::collections::HashMap;
use std::fs;
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::process::{Command, exit};

mod config;
use config::*;

mod locate_hwmon;
use locate_hwmon::prep_config;

fn main() {
    //get the path to the config file
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        println!("Usage: epycd8-fan-control <config_file_path>");
        exit(1);
    }
    let cfg_path = &args[1];
    println!("Loading config");
    let cfg_file = fs::read_to_string(cfg_path).expect("Config not found at path");
    let mut config: Config = serde_json::from_str(&cfg_file.as_str()).expect("Could not parse config file");
    
    // look at each temperature source
    // if it already has a path, leave it
    // otherwise use the sensor spec to find the path to the temperature file and insert it in the config
    prep_config(&mut config);


    println!("Starting fan control loop");

    //start the main loop
    runner(&config);
}

//wake up every n ms
fn runner(cfg: &Config) {
    let interval = Duration::from_millis(cfg.interval_ms.into());
    let mut next_time = Instant::now() + interval;
    loop {
        let temps = get_temps(cfg);
        control_fans(cfg, &temps);
        sleep(next_time - Instant::now());
        next_time += interval;
    }
}

// read every temperature source
fn get_temps(cfg: &Config) -> HashMap<String, u32> {
    let mut res = HashMap::new();
    for t in cfg.temp_sources.iter() {
        let path = t.path.as_ref().expect("Path was never populated");
        let prov_temp = get_temp_from_file(&path);
        // println!("{} => {}", &t.name, prov_temp);
        let temp = if prov_temp < 10 {
            eprintln!("Temp < 10C ({}), assuming we are reading from a source in C instead of milliC, failsafing to max fans", prov_temp);
            101
        }
        else {
            prov_temp
        };


        res.insert(t.name.clone(), temp);
    }
    res
}

//returns in degrees C
fn get_temp_from_file(path: &String) -> u32 {
    let contents = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to read temp source at \"{}\" ({}), defaulting to reporting 101C to the fans as a failsafe", path, e);
            return 101;
        }
    };
    //if we fail to parse the number, make the system believe the CPU is 101C as a failsafe
    match contents.trim().parse::<u32>() {
        Ok(v) => v / 1000,
        Err(e) => {
            eprintln!("Error parsing temp at \"{}\" ({}), defaulting to reporting 101C to the fans as a failsafe", path, e);
            101
        }
    }
} 

//compute the duty cycle for each fan and execute the command to run it
fn control_fans(cfg: &Config, temps: &HashMap<String, u32>) {
    let cpu = compute_dcycle(temps, &cfg.cpu_fan, "CPU1_FAN1");
    let frnt1 = compute_dcycle(temps, &cfg.frnt_fan1, "FRNT_FAN1");
    let frnt2 = compute_dcycle(temps, &cfg.frnt_fan2, "FRNT_FAN2");
    let frnt3 = compute_dcycle(temps, &cfg.frnt_fan3, "FRNT_FAN3");
    let frnt4 = compute_dcycle(temps, &cfg.frnt_fan4, "FRNT_FAN4");
    let rear1 = compute_dcycle(temps, &cfg.rear_fan1, "REAR_FAN1");
    let rear2 = compute_dcycle(temps, &cfg.rear_fan1, "REAR_FAN2");
    exec_command(cpu, frnt1, frnt2, frnt3, frnt4, rear1, rear2);
}

// this just does the fan curve math
fn compute_dcycle(temps: &HashMap<String, u32>, fan: &FanConfig, fan_name: &str) -> u32 {
    let temp = match temps.get(&fan.temp_source_name) {
        Some(t) => *t,
        None => {
            eprintln!("Temp source {} not found for fan {}, defaulting to max duty cycle", fan.temp_source_name, fan_name);
            101
        }
    };

    if fan.ramp.len() == 0 {
        eprintln!("NO RAMP STEPS FOUND FOR FAN {}, DEFAULTING TO MAX", fan_name);
        return 100;
    }

    let result = fan.ramp.binary_search_by(|s| s.temp_c.cmp(&temp));

    //taken from https://github.com/chenxiaolong/ipmi-fan-control/blob/master/src/main.rs
    // Index of first step >= the current temperature (if exists)
    let above_index = match result {
        Ok(i) => Some(i),
        Err(i) if i == fan.ramp.len() => None,
        Err(i) => Some(i),
    };
    // Index of first step < the current temperature (if exists)
    let below_index = match above_index {
        Some(0) => None,
        Some(i) => Some(i - 1),
        None => None,
    };

    // If step above doesn't exist, use last step's dcycle or 100%
    let above_step = match above_index {
        Some(i) => fan.ramp[i].clone(),
        None => {
            let duty_cyc = fan.ramp.last()
                .map_or(100, |s| s.duty_cyc);

            RampStep {
                temp_c: temp,
                duty_cyc,
            }
        }
    };
    // If step below doesn't exist, use same step as step above
    let below_step = match below_index {
        Some(i) => fan.ramp[i].clone(),
        None => above_step.clone(),
    };

    let dcycle_new = if below_step.temp_c == above_step.temp_c {
        below_step.duty_cyc
    } else {
        // Linearly scale the dcycle
        u32::from(temp - below_step.temp_c)
            * u32::from(above_step.duty_cyc - below_step.duty_cyc)
            / u32::from(above_step.temp_c - below_step.temp_c)
            + u32::from(below_step.duty_cyc)
    };

    dcycle_new

}

// takes each fan power as an integer between 0 and 100 representing the duty cycle
fn exec_command(cpu: u32, frnt1: u32, frnt2: u32, frnt3: u32, frnt4: u32, rear1: u32, rear2: u32) {
    //println!("Running command: raw 0x3a 0x01 0x{:x} 0x00 0x{:x} 0x{:x} 0x{:x} 0x{:x} 0x{:x} 0x{:x}", cpu, rear1, rear2, frnt1, frnt2, frnt3, frnt4);
    match Command::new("ipmitool")
        .args([
            "raw",
            "0x3a",
            "0x01",
            format!("0x{:x}", cpu).as_str(),
            "0x00",
            format!("0x{:x}", rear1).as_str(),
            format!("0x{:x}", rear2).as_str(),
            format!("0x{:x}", frnt1).as_str(),
            format!("0x{:x}", frnt2).as_str(),
            format!("0x{:x}", frnt3).as_str(),
            format!("0x{:x}", frnt4).as_str(),
        ])
        .output() {
            Ok(_) => (),
            Err(e) => { eprintln!("Error running ipmitool: {}", e); }
        }
}