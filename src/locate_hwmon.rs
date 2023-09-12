use std::{path::PathBuf, process::exit};

use crate::config::{Config, TempSource};

// take the configuration file and check each temp source
// if it doesn't have a path, try and locate the path to the sensor file by reading the sensor spec
// if it doesn't have a path or a sensor spec, error out
pub fn prep_config(cfg: &mut Config) {
    for temp_src in cfg.temp_sources.iter_mut() {
        match (temp_src.path.as_ref(), temp_src.sensor.as_ref()) {
            //if we have both a sensor and path spec, use the path
            (Some(path), Some(sens)) => {
                eprintln!("Temp source {} has both a path and sensor specified, using path", &temp_src.name);
            },
            //if we just have a path, we don't need to do anything
            (Some(path), None) => (),
            //if we just have a sensor, locate the path for it and update the config
            (None, Some(s)) => {
                println!("=============");
                println!("Looking for device {}, sensor {}", &s.device_name, &s.sensor_name);
                let path = get_path_to_temperature_file(&s.device_name, &s.sensor_name).expect("Could not find path for sensor");
                temp_src.path = Some(path);
            },
            //if we have neither a path nor a sensor, error out
            (None, None) => {
                eprintln!("You need to specify either a path or sensor for {}", temp_src.name);
                exit(1);
            }
        }
    }
}

// find the hwmon path whose name corresponds with the sensor spec
fn get_path_to_temperature_file(dev_name: &String, label: &String) -> Option<String> {
    let hwmon_paths = enumerate_hwmons();
    for path in hwmon_paths.iter() {
        if test_if_device(path, dev_name) {
            println!("Found device \"{}\" at \"{}\"", dev_name, path.to_str().unwrap());
            return find_sensor(path, label);
        }
    }
    None
}

// list out the hwmon folders
fn enumerate_hwmons() -> Vec<PathBuf> {
    let base_path = "/sys/class/hwmon";
    let paths = std::fs::read_dir(base_path).expect("Unable to read /sys/class/hwmon");

    paths.map(|dir|{ 
        match dir {
            Ok(dir) => Some(dir.path()),
            Err(e) => {
                eprintln!("Unable to read a folder in /sys/class/hwmon: {}", e);
                None
            } 
        }
    })
    .filter(|o| o.is_some())
    .map(|o| o.unwrap())
    .collect()
}

//test to see if the given hwmon folder contains the desired device
fn test_if_device(hwmon_path: &PathBuf, desired_device: &String) -> bool {
    let name_path = hwmon_path.join("name");
    match std::fs::read_to_string(&name_path) {
        Ok(n) => n.trim().eq(desired_device),
        Err(e) => {
            eprintln!("Failed to read device name at {}", name_path.to_str().unwrap());
            false
        }
    }
}


//within an hwmon folder, find with tempN_label matches the sensor name and return the tempN_input path
fn find_sensor(hwmon_dev_path: &PathBuf, desired_sensor: &String) -> Option<String> {
    let paths = std::fs::read_dir(hwmon_dev_path).expect(format!("Unable to read {}", hwmon_dev_path.to_str().unwrap()).as_str());
    for path in paths {
        match path {
            Err(e) => {
                eprintln!("Unable to read file in {} ({})", hwmon_dev_path.to_str().unwrap(), e);
            },
            Ok(p) => {
                let fname = p.file_name();
                let fn_string = fname.to_str().expect("Unable to convert file name to string");
                //only read tempN_labels
                if fn_string.starts_with("temp") && fn_string.ends_with("_label") {
                    //check if the text in tempN_label matches the desired sensor
                    let is_dev = match std::fs::read_to_string(p.path()) {
                        Ok(n) => n.trim().eq(desired_sensor),
                        Err(e) => {
                            eprintln!("Failed to sensor label at {}", p.path().to_str().unwrap());
                            false
                        }
                    };
                    //if it does, return the path to tempN_input
                    if is_dev {
                        let index_of_uscore = fn_string.find("_").expect("Underscore not found in sensor file name");
                        let (head, rest) = fn_string.split_at(index_of_uscore);
                        let (_, num_str) = head.split_at(4);
                        println!("For {}, found sensor \"{}\" in temp{}_input", hwmon_dev_path.to_str().unwrap(), desired_sensor, num_str);
                        return Some(String::from(hwmon_dev_path.join(format!("temp{}_input", num_str)).to_str().unwrap()));
                    }
                }
            }
        }
    }
    //we couldn't find it
    None
}