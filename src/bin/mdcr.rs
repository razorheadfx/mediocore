#![feature(drain_filter)]
use std::path::PathBuf;
use std::io;
use std::fs;

#[macro_use]
extern crate log;
extern crate mediocore;
extern crate env_logger;


macro_rules! ghz{
    ($x:expr) =>(   
        let ghz = $x as f64 / 1e6f64;
        format!("{:1.3} GHz", ghz)
)
}

	
fn main() -> io::Result<()>{
    env_logger::init();

    let cores = mediocore::discover_core_settings()?;
	debug!("Found Configuration {:#?}", cores);

	info!("Current Configuration:");
	 
    Ok(())
}
