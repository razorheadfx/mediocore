#[macro_use]
extern crate log;
extern crate env_logger;

extern crate structopt;
use std::fmt::Write;
use std::process::exit;
use mediocore::CoreSetting;
use structopt::StructOpt;

extern crate mediocore;

use std::io;
use std::io::ErrorKind;
use std::collections::HashSet;

#[derive(Debug, StructOpt)]
#[structopt(name = "mediocore")]
enum Mdcr{
    #[structopt(name = "set")]
    /// Set scaling governor and min/max scaling frequency. Run ```mdcr set help``` for details.
    Set(Cfg),
	#[structopt(name = "powersave")]
    /// Set maximum scaling frequency to minimal CPU frequency and set powersave governor.
	Powersave,
	#[structopt(name = "performance")]
    /// Set maximum scaling frequency to maximum CPU frequency and set performance governor.
	Performance,
	#[structopt(name = "show")]
    /// Pretty print per-core config
	Show
}

#[derive(Debug, StructOpt)]
#[structopt(name = "performance")]
struct Cfg{
    #[structopt(short="g",long="governor")]
    /// Change the scaling governor settings
    pub governor : Option<String>,
    /// Change the low/min scaling frequency 
    #[structopt(short="l",long="low")]
    pub min : Option<u32>,
    /// Change the high/max scaling frequency
    #[structopt(short="h",long="high")]
    pub max : Option<u32>,
    #[structopt(short="c",long="cores")]
    /// Comma separated cores to apply the settings to
    pub cores : Vec<u32>

}

macro_rules! try_or_exit{
    ($x:expr,$code:expr) => (match $x{
    	Ok(o) => o,
        Err(ref e) if e.kind() == ErrorKind::PermissionDenied =>{
            eprintln!("Permission denied. Do you have write access to /sys/devices/system/cpu/?");
            exit(13)
        },
    	Err(e) => {
    		eprintln!("Unexpected Error: {:?}",e);
    		exit($code)
    	}
    })
}

fn discover() -> io::Result<Vec<CoreSetting>>{
	let mut cores = mediocore::discover_core_settings()?;
    cores.sort_by_key(|c| c.num());
	debug!("Discovered Configuration {:#?}", cores);
	Ok(cores)
}

fn powersave(){
	unimplemented!()
}

fn performance(){
	unimplemented!()
}

fn set(cfg : Cfg){
    let mut cores = try_or_exit!(discover(),1);
    
    // cores specified? well then drop the others
    if !cfg.cores.is_empty(){
        cores.retain(|c|cfg.cores.iter().any(|n|n.eq(&c.num())));
    }

    let coreset = match cfg.governor{
        Some(gov) => {
            info!("Setting governor");
            cores.iter_mut().try_for_each(|c|c.set_governor(&gov))
        },
        None => {
            debug!("No governor settings to apply");
            Ok(())
        }
        
    };
    try_or_exit!(coreset, 1);

    let minset = match cfg.min{
        Some(min) =>{
            info!("Setting minimum frequencies");
            cores.iter_mut().try_for_each(|c|c.set_min(min*1000))
        },
        None => {
            debug!("No min settings to apply");
            Ok(())
        }
    };

    try_or_exit!(minset, 1);

    let maxset = match cfg.max{
        Some(max) =>{
            info!("Setting maximum frequencies");
            cores.iter_mut().try_for_each(|c|c.set_max(max*1000))
        },
        None => {
            debug!("No max settings to apply");
            Ok(())
        }
    };

    try_or_exit!(maxset, 1);

}

fn show(){
    /// Expected terminal line length
    const TERM_LEN : usize = 80;
    /// Width of the table 
    const TABLE_LEGEND_LEN : usize = 23;

	let cores = try_or_exit!(discover(),1);

    // find out how long the governor description is, then scale space alotted to each core accordingly
    let longest_gov = cores.iter().map(|c| c.curr_gov().len()).max().expect("No governors");
    // add 3 chars of padding (1 front, 1 end, 1 for the separator)
    let per_core_chars = longest_gov+3;
    let cores_per_line = (TERM_LEN-TABLE_LEGEND_LEN)/per_core_chars;

    // generate lines of core descriptions

    println!("Current Settings:");

    for cs in cores.chunks(cores_per_line){
    	let mut creline : String = "Core                   ".into();
    	let mut minline : String = "Min CPU/Current [GHz]  ".into();
    	let mut maxline : String = "Max CPU/Current [GHz]  ".into();   
    	let mut govline : String = "Current Governor       ".into();

    	for core in cs.iter(){
            let mut pad_to = creline.len()+per_core_chars;
    		write!(creline," {}", core.num());
    		write!(minline," {:03.3}/{:03.3}", core.cpu_min() as f64 / 1e6, core.curr_min() as f64 / 1e6);
            write!(maxline," {:03.3}/{:03.3}", core.cpu_max() as f64 / 1e6, core.curr_max() as f64 / 1e6);
            write!(govline," {}", core.curr_gov());
            for line in [&mut creline, &mut minline, &mut maxline, &mut govline].iter_mut(){
            while line.len() < pad_to{
                write!(line, " ");
            }
            // not a normal vertical line but box drawing character U+2502
            // https://en.wikipedia.org/wiki/Box-drawing_character
            write!(line,"│");    
        }

    	}
        
    	println!("{}",creline);
    	println!("{}",minline);
    	println!("{}",maxline);
    	println!("{}",govline);

        let mut divider = String::with_capacity(TERM_LEN);
        (0..creline.len()-8).for_each(|i| {
            if i < TABLE_LEGEND_LEN{
                divider.push_str(" ");
            }else{
                // not a normal dash but box drawing character U+2500
                // also longer than normal
                divider.push_str("─"); 
            }
        });
        println!("{}", divider);
        
    }

    // display all available governors
    //OPT: maybe use color coding so we can show which core supports which (most likely they are not going to differ... so why bother with the terminal color crate)
    let available_governors = cores.iter().fold(HashSet::new(),|mut govs, c|
    { 
        c.available_govs()
        .iter()
        .for_each(|g| {let _ = govs.insert(g);});
        govs
    });

    let av_govs = available_governors.iter().fold("* Available Governors   ".to_string(), | mut av_govs, gov|{
        write!(av_govs, "{} ", gov);
        av_govs
    });
    println!("{}",av_govs);

    // all good? exit with 0
    exit(0);


}

fn main(){
    env_logger::init();
    let settings = Mdcr::from_args();
    debug!("Args provided: {:#?}", settings);
    
    
    match settings{
            Mdcr::Set(c) => set(c),
            Mdcr::Powersave => powersave(),
            Mdcr::Performance => performance(),
            Mdcr::Show => show(),
    };   
}
