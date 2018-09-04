# mediocore - a more convenient cpufrequtils

## What is mediocore?
mediocore is a Rust implementation of the ```cpufrequtils``` toolkit used to get and set per-CPU-core frequency scaling governor and CPU frequency target.  
Like the original cpufrequtils, it uses Linux' sysfs to retrieve and manipulate the current CPU governor parameters located under /sys/devices/system/cpu/cpu<x>/.  
The main grief with cpufrequtils is that it only operates on single cores, so it must be wrapped in scripts (for example to set all of your cores to minimum frequency for maximum battery lifetime).  
mediocores' ```mdcr``` command provides convenient shortcuts to set parameters for all cores or display current settings.
Mediocore also sanity checks inputs by first discovering current and viable settings.
In addition, it provides an interactive mode which can be used to configure these settings via a CLI.

## Installing
With Rust installed run ```cargo install mediocore``` to install the mdcr utility.

## Usage
Run ```mdcr help``` to show available commands.  
* ```mdcr -g <governor>```  
* ```mdcr -f <frequency>``` tries to set the frequency in MHz - understands both integer form (i.e. 2000 for 2GHz) or float input (i.e. 2.5e3 for 2.5GHz) - if the value is out of bounds it logs an error and exits with code 1.  
By default mediocore will try to apply settings to all cores unless you specify an individual core e.g. ```mdcore -f 2.0e6 -c 1,5``` - if the specified cores could not be found it will do nothing and exit with code 1.  

### Shortcuts  
* ```mdcr powersave``` applies minimum scaling frequency and powersave governor - if available, else leaves governor unchanged, warns and exits with code 1).  
* ```mdcr performance``` applies maximum scaling frequency and performance governor - if avalable, else leaves governor unchanged, warns and exits with code 1).  

## License
Licensed under [MPL2](https://www.mozilla.org/en-US/MPL/2.0/).
See LICENSE for details.

## TODO
- [ ] Implement simple mode
- [ ] Implement interactive mode

