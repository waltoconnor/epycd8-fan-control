# epycd8-fan-control
The ASRock Rack EpycD8 motherboard does not have any configurable fan control settings as of BIOS 2.4, and must be controlled via IPMI.
This is a small rust program that reads temperatures from files (namely `/sys/class/hwmon/...`), uses these to determine fan duty cycles given a configuration file, and uses `ipmitool` to control the fan speeds.

This is designed specifically for EpycD8 motherboards, but would likely be trivial to adapt to other systems. 
I wrote this after trying to adapt the much more robust [ipmi-fan-control](https://github.com/chenxiaolong/ipmi-fan-control) project to my EpycD8, which ended up not working well. The Supermicro board `ipmi-fan-control` targets sets each fan duty cycle one at a time, and this is accomplished by running the logic for each fan/set of fans in separate threads while the EpycD8 sets them all in a single command, and there isn't a way to apply partial updates. It ended up being easier to just start from scratch.

# Requirements
This shouldn't require anything beyond the standard cargo/rustc toolchain to compile. It depends on having `ipmitool` available in the PATH, ensure that it is installed and available in the context this program will run in.
This program probably needs to be run as an administrator.

# Config
An example config can be found in `config.json`, which shows a system where `FRNT_FAN1` is used to cool the GPU and all other fans cool the CPU.
`interval_ms` is how often to update the fan speeds, in milliseconds.
`temp_sources` is a list of files to read temperatures from (in milli-celsius, which is standard for `/sys/class/hwmon/...`), and names to alias them as. You can either specify a path directly, or you can specify a sensor specification. See "Finding Temperature Sources" for more.


Each fan gets it's own fan configuration section.
Each fan configuration section indicates which temperature source to read from, and a `ramp` field, which is a list of temperatures and duty cycles defining a fan curve.
The `ramp` sections can have as many elements as you'd like, but they must be ordered by the `temp_c` field, from lowest to highest. 
For a temperature, the system will find the `ramp` elements that bound that temperature and linearly interpolate the duty cycles. For example, if you have:
`[{"temp_c": 50, "duty_cyc": 30}, {"temp_c": 100, "duty_cyc": 100}]`
An input temp of 75C will yield a duty cycle of 65%. If the temp falls off the scale, it will use the duty cycle at the closest boundary.

## Finding Temperature Sources
This program expects to read from `/sys/class/hwmon/...`. Unfortunately, the numbering of `hwmon0`, `hwmon1`, etc. is not stable between reboots and is very likely to change during kernel updates, so we need a system to automagically find the devices. 
Each folder in `/sys/class/hwmon` (i.e. `/sys/class/hwmon/hwmon0`) has a file called `name`, which contains the name of the driver that is reporting the temperature, and a set of files called `tempN_label`, which is the name of a sensor, and `tempN_input`, which is the temperature the sensor is reporting in milliCelsius. For instance, my GPU appears in `/sys/class/hwmon/hwmon0`, `name` contains "amdgpu", and `temp1_label` is "edge", `temp2_label` is "junction" and `temp3_label` is "memory". The respective `tempN_inputs` contain the respective temperatures. If I want to read the gpu junction temperature as the "gpu" temperature input, I would add `{"name": "gpu", "sensor": { "device_name": "amdgpu", "sensor_name": "junction" }}` as an entry in the `"temp_sources"` section of the config file. AMD CPU temps are nominally found under the `k10temp` device, with the `Tctl` sensor being what the CPU would like to report to the cooling system.
IF you have multiple devices using the same driver (i.e. you have multiple GPUs or are trying to use NVMe devices as temperature source), I do not have a good way to support that at this time. If you would like to manually specify a path to a temperature file, you can do that by setting the `path` field of the temperature input instead of the `sensor` field. You can hack your own sensor reading logic together by having a shell script using `lm_sensors` or something dump raw values in to some files, and then have this program read those files to control the fans, but at that point you should probably just write your own shell script to do everything.

# Usage
`# cargo run path/to/config.json`
or
`# epycd8-fan-control path/to/config.json`

Remember that you need to run it with admin rights.
It is recommended to run this program via a systemd unit, use the `Path` variable to make `ipmitool` available.

# Failure Cases
If there is a problem with the configuration file or finding the path to a sensor, the program will print an error and quit, SO CHECK THAT THE PROGRAM STARTED SUCCESSFULLY BEFORE LEAVING IT UNATTENDED.
In all cases where something uncouth happens while trying to read and parse the temperatures, the system will default to reporting 101C, which should cause any reasonable fan curve to max out.
In all cases where something goes wrong while parsing a fan curve, the system should run the fan at 100% duty cycle.
The system does not check that the fan curves you provide are actually sufficient to cool the system, I recommend always topping every fan curve off by maxing the duty cycle at 90C. It is difficult to kill modern computers by overheating them, but if you find a way to, I take no responsibility.
I hacked this program together in about an hour, so don't expect it to be rock solid.

# References
Lots of information on EpycD8 fan control can be found on the [STH forums](https://forums.servethehome.com/index.php?threads/asrock-rack-bmc-fan-control.26941/)
I use the mapping from raw byte to fan figured out by eduncan911 in this program.