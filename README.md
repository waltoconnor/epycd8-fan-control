# epycd8-fan-control
The ASRock Rack EpycD8 motherboard does not have any configurable fan control settings as of BIOS 2.4, and must be controlled via IPMI.
This is a small rust program that uses reads temperatures from files (namely `/sys/class/hwmon/...`), takes a configuration file, and uses `ipmitool` to control the fan speeds.

This is designed specifically for EpycD8 motherboards, but would likely be trivial to adapt to other systems. 
I wrote this after trying to adapt the much more robust [ipmi-fan-control](https://github.com/chenxiaolong/ipmi-fan-control) project to my ASRock motherboard, which ended up not working well as the supermicro board `ipmi-fan-control` targets sets each fan duty cycle one at a time in separate threads while the EpycD8 sets them all in a single command.

# Requirements
This shouldn't require anything beyond the standard cargo/rustc toolchain to compile. It depends on having `ipmitool` available in the PATH, ensure that it is installed and available in the context this program will run in.
This program probably needs to be run as an administrator.

# Config
An example config can be found in `config.json`, which shows a system where `FRNT_FAN1` is used to cool the GPU and all other fans cool the CPU.
`interval_ms` is how often to update the fan speeds, in milliseconds.
`temp_sources` is a list of files to read temperatures from (in milli-celsius, which is standard for `/sys/class/hwmon/...`), and names to alias them as.
Each fan gets it's own fan configuration section.
Each fan configuration section indicates which temperature source to read from, and a `ramp` field, which is a list of temperatures and duty cycles defining a fan curve.
The `ramp` sections can have as many elements as you'd like, but they must be ordered by the `temp_c` field, from lowest to highest. 
For a temperature, the system will find the `ramp` elements that bound that temperature and linearly interpolate the duty cycles. For example, if you have:
`[{"temp_c": 50, "duty_cyc": 30}, {"temp_c": 100, "duty_cyc": 100}]`
An input temp of 75C will yield a duty cycle of 65%. If the temp falls off the scale, it will use the duty cycle at the closest boundary.

# Usage
`# cargo run path/to/config.json`
or
`# epycd8-fan-control path/to/config.json`

Remember that you need to run it with admin rights.

# Failure Cases
In all cases where something uncouth happens while trying to read and parse the temperatures, the system will default to reporting 101C, which should cause any reasonable fan curve to max out.
In all cases where something goes wrong while parsing a fan curve, the system should run the fan at 100% duty cycle.
The system does not check that the fan curves you provide are actually sufficient to cool the system, I recommend always topping every fan curve off by maxing the duty cycle at 90C. It is difficult to kill modern computers by overheating them, but if you find a way to, I take no responsibility.
I hacked this program together in about an hour, so don't expect it to be rock solid.

# References
Lots of information on EpycD8 fan control can be found on the [STH forums](https://forums.servethehome.com/index.php?threads/asrock-rack-bmc-fan-control.26941/)
I use the mapping from raw byte to fan figured out by eduncan911 in this program.