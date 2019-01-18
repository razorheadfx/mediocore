#[macro_use]
extern crate log;

use std::fs;
use std::io;
use std::io::{Error, ErrorKind, Write};
use std::path::PathBuf;

macro_rules! parse_num {
    ($g:ident, $op:expr) => {{
        let mut chars = fs::read_to_string($g.join($op))?;
        chars.retain(|c| c.is_digit(10));

        match chars.parse() {
            Ok(x) => x,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Cause{}", e),
                ));
            }
        }
    }};
}

/// find relevant sysfs folders in /sys/devices/system/cpu/cpu<x>
pub fn discover_core_settings() -> io::Result<Vec<Core>> {
    let cpu_root = fs::read_dir("/sys/devices/system/cpu/")?;
    debug!("Content of /sys/devices/system/cpu/  {:#?}", cpu_root);

    let is_core = |p: &fs::DirEntry| {
        let f = p
            .file_name()
            .into_string()
            .expect("Encountered invalid path while discovering core directories");
        f.contains("cpu") && !(f.contains("cpuidle") || f.contains("cpufreq"))
    };

    cpu_root
        // WARN: error cases described by read dir seem unrealistic at first so we're gonna ignore them
        .filter_map(|e| e.ok())
        .filter(|p| is_core(p))
        .map(|p| p.path())
        .inspect(|c| debug!("Found core: {:?}", c))
        .try_fold(Vec::new(), |mut cores, c| {
            let c = Core::discover(c)?;
            cores.push(c);
            Ok(cores)
        })
}

/// Representation of the current cpufrequency serttings of a single core
/// Can be obtained by running [discover_core_settings]
#[derive(Clone, Debug)]
pub struct Core {
    /// Path to the core directory
    core: PathBuf,
    /// Number of the core
    num: u32,
    /// CPU Minimum Frequency
    cpuinfo_max_freq: u32,
    /// CPU Maximum Frequency
    cpuinfo_min_freq: u32,
    /// List of possible values for the governor
    scaling_available_governors: Vec<String>,
    /// Current upper frequency limit - the governor may increase frequency up to this value
    scaling_max_freq: u32,
    /// Current lower frequency limit - the governor may reduce the frequency down to this value
    scaling_min_freq: u32,
    /// Currently set scaling governor
    scaling_governor: String,
}

impl Core {
    /// discover settings for the core specified by its path
    pub fn discover(core: PathBuf) -> io::Result<Core> {
        let g = core.join("cpufreq");

        let cpuinfo_min_freq: u32 = parse_num!(g, "cpuinfo_min_freq");
        let cpuinfo_max_freq: u32 = parse_num!(g, "cpuinfo_max_freq");
        let scaling_min_freq: u32 = parse_num!(g, "scaling_min_freq");
        let scaling_max_freq: u32 = parse_num!(g, "scaling_max_freq");
        let scaling_governor = {
            let mut chars = fs::read_to_string(g.join("scaling_governor"))?;
            chars.retain(|c| !c.is_control());
            chars
        };

        let scaling_available_governors = {
            let mut chars = fs::read_to_string(g.join("scaling_available_governors"))?;
            chars.retain(|c| !c.is_control());
            chars.split(&" ").map(|s| s.into()).collect()
        };

        // parse the number
        let num = core
            .to_str()
            .expect("Failed to convert PathBuf to String")
            .rsplit("cpu")
            .next()
            .expect("Core name did not contain 'cpu'")
            .parse()
            .expect("Failed to parse the u32 core number");

        let c = Core {
            core,
            num,
            cpuinfo_max_freq,
            cpuinfo_min_freq,
            scaling_available_governors,
            scaling_governor,
            scaling_min_freq,
            scaling_max_freq,
        };
        debug!("Read settings : {:#?}", c);

        Ok(c)
    }

    /// returns the number of the core
    pub fn num(&self) -> u32 {
        self.num
    }

    /// returns cpu minimum frequency in kHz
    pub fn cpu_min(&self) -> u32 {
        self.cpuinfo_min_freq
    }

    /// returns cpu maximum frequency in kHz
    pub fn cpu_max(&self) -> u32 {
        self.cpuinfo_max_freq
    }

    /// returns current min scaling frequency in kHz
    pub fn curr_min(&self) -> u32 {
        self.scaling_min_freq
    }

    /// returns current max scaling frequency in kHz
    pub fn curr_max(&self) -> u32 {
        self.scaling_max_freq
    }

    /// returns the current governor
    pub fn curr_gov(&self) -> &str {
        self.scaling_governor.as_ref()
    }

    /// returns available governors
    pub fn available_govs(&self) -> &[String] {
        self.scaling_available_governors.as_ref()
    }

    /// Validate the given minimum value. Must be >= the discovered CPU frequency minimum
    pub fn validate_min(&self, freq: u32) -> io::Result<u32> {
        if self.cpuinfo_min_freq <= freq && freq <= self.scaling_max_freq {
            Ok(freq)
        } else {
            Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Min Frequency {} not in ({},{}]",
                    freq, self.cpuinfo_min_freq, self.scaling_max_freq
                ),
            ))
        }
    }

    /// Validate the given maximum value. Must be >= current min and <= CPU frequency maximum
    pub fn validate_max(&self, freq: u32) -> io::Result<u32> {
        if (self.cpuinfo_min_freq <= freq && self.scaling_min_freq <= freq)
            && freq <= self.cpuinfo_max_freq
        {
            Ok(freq)
        } else {
            Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Max Frequency {} not in min({},{})..={}",
                    freq, self.cpuinfo_min_freq, self.scaling_min_freq, self.cpuinfo_max_freq
                ),
            ))
        }
    }

    /// Validate the governor by checking against the list of available governors
    pub fn validate_governor<'a>(&self, governor: &'a str) -> io::Result<&'a str> {
        if self
            .scaling_available_governors
            .iter()
            .any(|g| g.as_str().eq(governor))
        {
            Ok(governor)
        } else {
            Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Governor {} not available. Must be one of {:?}",
                    governor, self.scaling_available_governors
                ),
            ))
        }
    }

    /// Set the minimum scaling frequency (lower frequency limit)
    /// This operation is not checked by mediocore, but the kernel may refuse to accept certain inputs.  
    /// Use [Core::validate_min] on the value beforehand.
    pub fn set_min(&mut self, freq: u32) -> io::Result<()> {
        debug!(
            "Setting minimum scaling frequency {} on {}",
            freq, self.num
        );
        let mut f = fs::OpenOptions::new()
            .write(true)
            .open(self.core.join("cpufreq/scaling_min_freq"))?;
        f.write_all(format!("{}", freq).as_ref())?;
        Ok(())
    }

    /// Set the maximum scaling frequency (lower frequency limit)  
    /// This operation is not checked by mediocore, but the kernel may refuse to accept certain inputs.  
    /// Use [Core::validate_max] on the value beforehand.
    pub fn set_max(&mut self, freq: u32) -> io::Result<()> {
        debug!(
            "Setting maximum scaling frequency {} on {}",
            freq, self.num
        );
        let mut f = fs::OpenOptions::new()
            .write(true)
            .open(self.core.join("cpufreq/scaling_max_freq"))?;
        f.write_all(format!("{}", freq).as_ref())?;
        Ok(())
    }

    /// Apply the given governor
    /// This operation is not checked by mediocore, but the kernel may refuse to accept certain inputs.
    /// Use [Core::validate_governor] on the value beforehand.
    pub fn set_governor(&mut self, guvnor: &str) -> io::Result<()> {
        debug!("Setting governor {} on {}", guvnor, self.num);
        fs::OpenOptions::new()
            .write(true)
            .read(false)
            .open(self.core.join("cpufreq/scaling_governor"))
            .and_then(|mut f| f.write_all(guvnor.as_bytes()))
            .map(|_| ())
    }
}

#[cfg(test)]
mod test {
    use io::{ErrorKind, Result};
    use std::path::PathBuf;
    use Core;

    #[test]
    fn freq_validation() {
        let s = Core {
            core: PathBuf::from("/sys/devices/system/cpu/cpu0"),
            num: 0,
            cpuinfo_min_freq: 800000,
            cpuinfo_max_freq: 2500000,
            scaling_available_governors: vec![],
            scaling_governor: "".into(),
            scaling_min_freq: 850000,
            scaling_max_freq: 900000,
        };

        let check_val = |x, v| match x {
            Ok(u) => v == u,
            Err(_) => false,
        };

        let check_err = |x: Result<u32>| match x {
            Ok(_) => false,
            Err(ref e) if e.kind() == ErrorKind::InvalidInput => true,
            _ => false,
        };

        assert!(check_val(s.validate_min(800000), 800000));
        assert!(check_val(s.validate_min(850000), 850000));
        assert!(check_err(s.validate_min(1000000)));

        assert!(check_val(s.validate_max(1000000), 1000000));
        assert!(check_err(s.validate_max(8000000)));
    }

    #[test]
    fn govnor_validation() {
        let s = Core {
            core: PathBuf::from(&"/sys/devices/system/cpu/cpu0"),
            num: 0,
            cpuinfo_min_freq: 800000,
            cpuinfo_max_freq: 2500000,
            scaling_available_governors: vec!["performance".into(), "powersave".into()],
            scaling_governor: "powersave".into(),
            scaling_min_freq: 850000,
            scaling_max_freq: 900000,
        };

        assert!(s.validate_governor(&"performance").is_ok());
        assert!(s.validate_governor(&"conservative").is_err());
    }

}
