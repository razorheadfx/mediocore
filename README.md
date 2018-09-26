# mediocore - a more convenient cpufrequtils

## What is mediocore?
mediocore is a Rust implementation of the ```cpufrequtils``` toolkit used to get and set per-CPU-core frequency scaling governor and CPU frequency target.  
Like the original cpufrequtils, it uses Linux' sysfs to retrieve and manipulate the current CPU governor parameters located under /sys/devices/system/cpu/cpu<x>/.  
The main grief with ```cpufreq-set``` is that it only operates on single cores, so it must be wrapped in scripts (for example to set all of your cores to minimum frequency for maximum battery lifetime).  
mediocores' ```mdcr``` command provides convenient shortcuts to set parameters for all cores or display current settings.
Mediocore also sanity checks inputs by first discovering current and viable settings.

## Installing
With Rust installed run ```cargo install mediocore``` to install the mdcr utility.

## Usage
Run ```mdcr help``` to show available commands.  
* ```mdcr set -g <governor>```  
* ```mdcr set -f <frequency>``` tries to set the maximum scaling frequency in MHz (i.e. 2000 for 2GHz) - if the value is out of bounds it logs an error and exits with code 1.  
By default mediocore will try to apply settings to all cores unless you specify an individual core (e.g. ```mdcore -f 2000 -c 1,5``` for cores 1 and 5) - if the specified cores could not be found it will do nothing and exit with code 1.  


### Shortcuts  
* ```mdcr powersave``` applies minimum scaling frequency and powersave governor - if available, else leaves governor unchanged, warns and exits with code 1).  
* ```mdcr performance``` applies maximum scaling frequency and performance governor - if available, else leaves governor unchanged, warns and exits with code 1).  

## License
Licensed under [MPL2](https://www.mozilla.org/en-US/MPL/2.0/).
See LICENSE for details.

## TODO
- [ ] Implement performance
- [ ] Implement powersave
- [ ] Implement set

## Optimisations
- [ ] Store core number in CoreSetting 
- [ ] Display available governors in show mode using color coding