use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub interval_ms: u32,
    pub temp_sources: Vec<TempSource>,
    pub cpu_fan: FanConfig,
    pub frnt_fan1: FanConfig,
    pub frnt_fan2: FanConfig,
    pub frnt_fan3: FanConfig,
    pub frnt_fan4: FanConfig,
    pub rear_fan1: FanConfig,
    pub rear_fan2: FanConfig,
    
}

// reads file in millicelsius
#[derive(Deserialize)]
pub struct TempSource {
    pub path: String,
    pub name: String
}

#[derive(Deserialize)]
pub struct FanConfig {
    pub temp_source_name: String,
    pub ramp: Vec<RampStep> //RampSteps need to be in order from lowest temp to highest temp
}

#[derive(Deserialize, Clone)]
pub struct RampStep {
    pub temp_c: u32,
    pub duty_cyc: u32
}