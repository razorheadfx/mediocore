# mediocore - a more convenient cpufrequtils
[![Latest Version](https://img.shields.io/crates/v/mediocore.svg)](https://crates.io/crates/mediocore)
[![Documentation](https://docs.rs/mediocore/badge.svg)](https://docs.rs/crate/mediocore)
![License](https://img.shields.io/crates/l/mediocore.svg)

## What is mediocore?
mediocore is a Rust implementation of the ```cpufrequtils``` toolkit used to get and set per-CPU-core frequency scaling governor and CPU frequency target.  
Like the original cpufrequtils, it uses Linux' sysfs to retrieve and manipulate the current CPU governor parameters located under /sys/devices/system/cpu/cpu<x>/.  
The main grief with ```cpufreq-set``` is that it only operates on single cores, so it must be wrapped in scripts (for example to set all of your cores to minimum frequency for maximum battery lifetime).  
mediocores' ```mdcr``` command provides convenient shortcuts to set parameters for all cores or display current settings.
Mediocore also sanity checks inputs by first discovering current and viable settings.

## Installing
With Rust installed run ```cargo install mediocore``` to install the mdcr utility.

## Usage
Run ```mdcr help``` to show available commands and ```mdcr help <subcommand>``` to see each subcommands help messages.

* ```mdcr show``` discovers and displays current/possible settings in a console friendly way
* ```mdcr show --json``` writes discovered settings to stdout as json  
* ```mdcr  set [-g governor] [-l lower_threshold] [-h upper_threshold] [-c comma_separated_list_of_core_numbers] ``` applies the settings given via -g/-l/-h to all cores unless a set of cores is specified via -c

There are also two shortcut commands:  
* ```mdcr ps|powersave``` sets cpu minimum frequency for both lower and upper frequency limits and applies powersave governor.  
* ```mdcr p|performance``` sets cpu maximum frequency as the upper frequency limit and applies performance governor.  

## License
Licensed under [MPL2](https://www.mozilla.org/en-US/MPL/2.0/).
See LICENSE for details.
