#[macro_use]
extern crate log;

use std::path::PathBuf;
use std::io::{Error, ErrorKind, Write};
use std::io;
use std::fs;


macro_rules! parse_num{
    ($g:ident, $op:expr) => ({
        let mut chars = fs::read_to_string($g.join($op))?;
        chars.retain(|c|c.is_digit(10));

        match chars.parse(){
            Ok(x) => x,
            Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Cause{}", e)))
        }
    
    })
}


/// find relevant sysfs folders in /sys/devices/system/cpu/cpu<x>
pub fn discover_core_settings() -> io::Result<Vec<CoreSetting>>{
    let cpu_root = fs::read_dir("/sys/devices/system/cpu/")?;
    debug!("Content of /sys/devices/system/cpu/  {:#?}", cpu_root);
    

    let is_core = |p : &fs::DirEntry|{
        let f = p.file_name()
            .into_string().expect("Encountered invalid path while discovering core directories");
        f.contains("cpu") && !(f.contains("cpuidle") || f.contains("cpufreq")) 
    };

    cpu_root
    // WARN: error cases described by read dir seem unrealistic at first so we're gonna ignore them
        .filter_map(|e|e.ok())
        .filter(|p| is_core(p))
        .map(|p|p.path())
        .inspect(|c|debug!("Found core: {:?}", c))
        .try_fold(Vec::new(), |mut cores, c| {
            let c = CoreSetting::discover(c)?;
            cores.push(c);
            Ok(cores)
    })
    
}




#[derive(Clone, Debug)]
pub struct CoreSetting{
    core : PathBuf,
    cpuinfo_max_freq : u32,
    cpuinfo_min_freq : u32,
    scaling_available_governors : Vec<String>,
    scaling_max_freq : u32,
    scaling_min_freq : u32,
    scaling_governor : String,
}

impl CoreSetting{


    /// discover settings for the core specified by its path
    pub fn discover(core: PathBuf) -> io::Result<CoreSetting>{
    let g = core.join("cpufreq");

    let cpuinfo_min_freq : u32 = parse_num!(g, "cpuinfo_min_freq");
    let cpuinfo_max_freq : u32 = parse_num!(g, "cpuinfo_max_freq");
    let scaling_min_freq : u32 = parse_num!(g, "scaling_min_freq");
    let scaling_max_freq : u32 = parse_num!(g, "scaling_max_freq");
    let scaling_governor = {
        let mut chars = fs::read_to_string(g.join("scaling_governor"))?;
        chars.retain(|c|!c.is_control());
        chars
    };

    let scaling_available_governors = {
        let mut chars = fs::read_to_string(g.join("scaling_available_governors"))?;
        chars.retain(|c|!c.is_control());
        chars.split(&" ").map(|s|s.into()).collect()
    };
    let c = CoreSetting{
        core,
        cpuinfo_max_freq,
        cpuinfo_min_freq,
        scaling_available_governors,
        scaling_governor,
        scaling_min_freq,
        scaling_max_freq
    };
    debug!("Read settings : {:#?}", c);

    Ok(c)
    }

    /// parse the core path to get the core number
    // OPT: should do this when discovering the cores
    pub fn num(&self) -> u32{
        self.core.file_name()
            .expect("Invalid core name")
            .to_str()
            .expect("Failed to convert to core name to str")
            .rsplit("cpu").next().expect("Core name did not contain 'cpu'")
            .parse()
            .expect("Core name parsing failed")
    }

    /// check given governor against valid governors and apply it to the internal representation
    /// Only calls to apply will write changes to the sysfs
    pub fn set_governor(&mut self, guvnor : &str) -> io::Result<()>{
        self.validate_governor(guvnor).and_then(|guvnor|{
            let mut f = fs::OpenOptions::new().write(true).open(self.core.join("cpufreq/scaling_min_freq"))?;
            f.write_all(guvnor.as_ref())?;
            Ok(())

        })
    }

    pub fn validate_min(&self, freq: u32) -> io::Result<u32>{
        if self.cpuinfo_min_freq <= freq && freq <= self.scaling_max_freq
        {
            Ok(freq)
        }else{
            Err(Error::new(ErrorKind::InvalidInput, format!("Min Frequency{} not in {}..={}", freq, self.cpuinfo_min_freq, self.scaling_max_freq)))
        }
    }

    pub fn validate_max(&self, freq: u32) -> io::Result<u32>{
        if  (self.cpuinfo_min_freq <= freq && self.scaling_min_freq <= freq)
             && freq <= self.cpuinfo_max_freq
        {
            Ok(freq)
        }else{
            Err(Error::new(ErrorKind::InvalidInput, format!("Max Frequency {} not in min({},{})..={}", freq, self.cpuinfo_min_freq, self.scaling_min_freq, self.cpuinfo_max_freq)))
        }
    }

    pub fn validate_governor<'a>(&self, governor : &'a str) -> io::Result<&'a str>{
        if self.scaling_available_governors.iter().any(|g|g.as_str().eq(governor)){
            Ok(governor)
        }else{
            Err(Error::new(ErrorKind::InvalidInput, format!("Governor {} not available. Must be one of {:?}", governor, self.scaling_available_governors)))
        }
    }

    pub fn set_min(&mut self, freq : u32) -> io::Result<()>{
        self.validate_min(freq).and_then(|freq|{
            let mut f = fs::OpenOptions::new().write(true).open(self.core.join("cpufreq/scaling_min_freq"))?;
            f.write_all(format!("{}",freq).as_ref())?;
            Ok(())
        })
    }


    pub fn set_max(&mut self, freq : u32) -> io::Result<()>{
        self.validate_max(freq).and_then(|freq|{
            let mut f = fs::OpenOptions::new().write(true).open(self.core.join("cpufreq/scaling_max_freq"))?;
            f.write_all(format!("{}",freq).as_ref())?;
            Ok(())
        })
    }
}


#[cfg(test)]
mod test{
    use CoreSetting;
    use std::path::PathBuf;
    use io::{ErrorKind, Result};
    
    #[test]
    fn freq_validation() {
        let s = CoreSetting{
            core : PathBuf::new(),
            cpuinfo_min_freq : 800000,
            cpuinfo_max_freq : 2500000,
            scaling_available_governors : vec![],
            scaling_governor : "".into(),
            scaling_min_freq : 850000,
            scaling_max_freq : 900000
        };

        let check_val = |x,v|{
            match x{
                Ok(u) => v == u,
                Err(_) => false
            }
        };

        let check_err = |x : Result<u32>|{
            match x{
                Ok(_) => false,
                Err(ref e) if e.kind() == ErrorKind::InvalidInput => true,
                _ => false
            }
        };

        assert!(check_val(s.validate_min(800000), 800000));
        assert!(check_val(s.validate_min(850000), 850000));
        assert!(check_err(s.validate_min(1000000)));

        assert!(check_val(s.validate_max(1000000), 1000000));
        assert!(check_err(s.validate_max(8000000)));

    }

    #[test]
    fn govnor_validation() {
        let s = CoreSetting{
            core : PathBuf::new(),
            cpuinfo_min_freq : 800000,
            cpuinfo_max_freq : 2500000,
            scaling_available_governors : vec!["performance".into(),"powersave".into()],
            scaling_governor : "powersave".into(),
            scaling_min_freq : 850000,
            scaling_max_freq : 900000
        };

        assert!(s.validate_governor(&"performance".into()).is_ok());
        assert!(s.validate_governor(&"conservative".into()).is_err());
    }


}

