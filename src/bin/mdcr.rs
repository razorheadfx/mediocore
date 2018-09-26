#[macro_use]
extern crate log;
extern crate env_logger;

#[macro_use]
extern crate structopt;
use std::fmt::Write;
use std::process::exit;
use mediocore::CoreSetting;
use structopt::StructOpt;

extern crate mediocore;

use std::io;
use std::io::{ErrorKind};
use std::collections::HashSet;

macro_rules! ghz{
    ($x:expr) =>(   
        let ghz = $x as f64 / 1e6f64;
        format!("{:1.3} GHz", ghz)
)
}

#[derive(Debug, StructOpt)]
#[structopt(name = "mediocore")]
enum Mdcr{
	#[structopt(name = "help", help = "print help messages")]
	Help,
	#[structopt(name = "powersave")]
	Powersave,
	#[structopt(name = "performance")]
	Performance,
	#[structopt(name = "show")]
	Show
}

macro_rules! try_or_exit{
    ($x:expr,$code:expr) => (match $x{
    	Ok(o) => o,
    	Err(e) => {
    		eprintln!("{:?}",e);
    		exit($code)
    	}
    })
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

    println!("Current Settings");

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
            Mdcr::Help => unimplemented!(),
            Mdcr::Powersave => powersave(),
            Mdcr::Performance => performance(),
            Mdcr::Show => show(),
            _ => unimplemented!()
    };   
}
