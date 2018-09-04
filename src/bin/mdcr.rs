#[macro_use]
extern crate log;
extern crate env_logger;

#[macro_use]
extern crate structopt;
use structopt::StructOpt;

extern crate mediocore;

use std::io;

macro_rules! ghz{
    ($x:expr) =>(   
        let ghz = $x as f64 / 1e6f64;
        format!("{:1.3} GHz", ghz)
)
}

#[derive(Debug, StructOpt)]
#[structopt(name = "mediocore")]
enum Mdcr{
	#[structopt(name = "help")]
	Help,
	#[structopt(name = "powersave")]
	Powersave,
	#[structopt(name = "performance")]
	Performance,
	#[structopt(name = "show"]
	Show
}

fn discover() -> io::Result<Vec<CoreSetting>>{
	let mut cores = mediocore::discover_core_settings()?;
    cores.sort_by_key(|c| c.num());
	info!("Discovered Configuration {:#?}", cores);
	Ok(cores)
}

fn powersave(){
	unimplemented!()
}

fn performance(){
	unimplemented!()
}

fn show(){
	unimplemented!()
}
	
fn main(){
    env_logger::init();
    let settings = Mdcr::from_args();
    debug!("Args provided: {:#?}", settings)
    
    
	match settings{
			Help => unimplemented!(),
			Powersave => powersave(),
			Performance => performance(),
			_ => unimplemented!()
	}
	 
    Ok(())
}
