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
        chars.split(" ").map(|s|s.into()).collect()
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

    /// check given governor against valid governors and apply it to the internal representation
    /// Only calls to apply will write changes to the sysfs
    pub fn set_governor(&mut self, guvnor : &String) -> io::Result<()>{
        if !self.scaling_available_governors.contains(guvnor){
            return Err(Error::new(ErrorKind::InvalidInput, format!("Invalid scaling governor, must be one of {:?}", self.scaling_available_governors)))
        }
        self.scaling_governor = guvnor.clone();
        Ok(())
    }

    pub fn set_max(&mut self, freq : u32) -> io::Result<()>{
        match freq{
            self.cpuinfo_min_freq..self.scaling_max_freq => unimplemented!(),
            _ => unimplemented!()
        }
    }
}
