#[macro_use]
extern crate log;
extern crate mediocore;
extern crate serde_json;
extern crate structopt;

use std::collections::HashSet;
use std::io::{stdout, ErrorKind, Write};
use std::process::exit;
use structopt::StructOpt;

use mediocore::Core;

#[derive(Debug, StructOpt)]
#[structopt(name = "mediocore", about = "discover and manipulate linux cpu frequency settings")]
enum Mdcr {
    #[structopt(name = "set")]
    /// Manipulate scaling governor and min/max scaling frequency. Run "mdcr set help" for details.
    Set(Cfg),
    #[structopt(name = "powersave")]
    /// Shortcut: sets low and high scaling frequency thresholds to minimum and applies powersave governor.
    Powersave,
    #[structopt(name = "performance")]
    /// Shortcut: sets high scaling frequency threshold to maximum and applies performance governor.
    Performance,
    #[structopt(name = "show")]
    /// Discover and show per-core settings either as console-friendly table or print the raw data as json via --json
    Show {
        #[structopt(long = "json", help = "print raw data as json")]
        json: bool,
    },
}

#[derive(Debug, StructOpt)]
#[structopt(name = "performance")]
struct Cfg {
    #[structopt(short = "g", long = "governor")]
    /// Apply the provided scaling governor.
    pub governor: Option<String>,
    /// Change the low/min scaling frequency threshold.
    #[structopt(short = "l", long = "low")]
    pub low: Option<u32>,
    /// Change the high/max scaling frequency threshold.
    #[structopt(short = "h", long = "high")]
    pub high: Option<u32>,
    #[structopt(short = "c", long = "cores")]
    /// Comma separated cores to apply the settings. If unspecified settings are applied to all cores.
    pub cores: Vec<u32>,
}

macro_rules! try_or_exit{
    ($x:expr, $msg:expr) => (match $x{
    	Ok(o) => o,
        Err(ref e) if e.kind() == ErrorKind::PermissionDenied =>{
            eprintln!("{}", $msg);
            eprintln!("Error: Permission denied.\n\tCause: {:#?}.\nDo you have write access to /sys/devices/system/cpu/?",e);
            exit(13)
        },
        Err(ref e) if e.kind() == ErrorKind::InvalidInput =>{
            eprintln!("{}", $msg);
            eprintln!("Error: Invalid input.\n\tCause: {:#?}.\nPlease check arguments.",e);
            exit(22)
        },
    	Err(e) => {
            eprintln!("{}", $msg);
    		eprintln!("Error: Unexpected Error:\n\tCause {:#?}.",e);
    		exit(1)
    	}
    })
}

fn discover_cores() -> Vec<Core> {
    let mut cores = try_or_exit!(
        mediocore::discover_core_settings(),
        "Failed to discover cores"
    );
    cores.sort_by_key(|c| c.num());
    debug!("Discovered Configuration {:#?}", cores);
    cores
}

fn powersave() {
    let mut cores = discover_cores();

    {
        let cores_no_psave = cores
            .iter()
            .filter(|c| {
                !c.available_govs()
                    .iter()
                    .any(|g: &String| "powersave".eq(g))
            })
            .map(|c| c.num())
            .collect::<Vec<_>>();
        if !cores_no_psave.is_empty() {
            eprintln!(
                "Cores {:?} do not support the powersave governor.",
                cores_no_psave
            );
            exit(1);
        }
    }

    debug!("Applying powersave governor on all cores");

    let res = cores
        .iter_mut()
        .try_for_each(|c| c.set_governor("powersave"));

    try_or_exit!(res, "Failed to set powersave governor");

    // set frequency to minimum
    let res = cores.iter_mut().try_for_each(|c| {
        let min = c.cpu_min();
        c.set_min(min).and(c.set_max(min))
    });

    try_or_exit!(res, "Failed to set scaling frequency to minimum");
    exit(0)
}

fn performance() {
    let mut cores = discover_cores();

    {
        let cores_no_perf = cores
            .iter()
            .filter(|c| {
                !c.available_govs()
                    .iter()
                    .any(|g: &String| "performance".eq(g))
            })
            .map(|c| c.num())
            .collect::<Vec<_>>();
        if !cores_no_perf.is_empty() {
            eprintln!(
                "Cores {:?} do not support the performance governor.",
                cores_no_perf
            );
            exit(1);
        }
    }

    debug!("Applying performance governor on all cores");

    let res = cores
        .iter_mut()
        .try_for_each(|c| c.set_governor("performance"));

    try_or_exit!(res, "Failed to set performance governor");

    // set frequency to minimum
    let res = cores.iter_mut().try_for_each(|c| {
        let max = c.cpu_max();
        c.set_max(max)
    });

    try_or_exit!(res, "Failed to set scaling frequency to maximum");
    exit(0)
}

fn set(cfg: Cfg) {
    let mut cores = discover_cores();


    if cfg.governor.is_none() && cfg.low.is_none() && cfg.high.is_none(){
        eprintln!("Please provide settings to set. Run \"mdcr help set\" to see the options");
        exit(1);
    }
    
    // cores specified? well then drop the others
    if !cfg.cores.is_empty() {
        cores.retain(|c| cfg.cores.iter().any(|n| n.eq(&c.num())));
    }

    match cfg.governor {
        Some(gov) => {
            info!("Setting governor");
            let res = cores.iter_mut().try_for_each(|c| c.set_governor(&gov));
            try_or_exit!(res, format!("Failed to set governor to {}", gov));
        }
        None => debug!("No governor settings to apply"),
    };

    match cfg.low {
        Some(min) => {
            info!("Setting minimum frequencies");
            let res = cores.iter_mut().try_for_each(|c| c.set_min(min * 1000));
            try_or_exit!(res, format!("Failed to set minimum frequency to {}", min));
        }
        None => debug!("No min settings to apply"),
    };

    match cfg.high {
        Some(max) => {
            info!("Setting maximum frequencies");
            let res = cores.iter_mut().try_for_each(|c| c.set_max(max * 1000));
            try_or_exit!(res, format!("Failed to set maximum frequency to {}", max));
        }
        None => debug!("No max settings to apply"),
    };

    exit(0)
}

fn print_pretty(cores: &[Core]) {
    /// Expected terminal line length
    const TERM_LEN: usize = 80;
    /// Width of the table
    const TABLE_LEGEND_LEN: usize = 23;

    // find out how long the governor description is, then scale space alotted to each core accordingly
    let longest_gov = cores
        .iter()
        .map(|c| c.curr_gov().len())
        .max()
        .expect("No governors");
    // add 3 chars of padding (1 front, 1 end, 1 for the separator)
    let per_core_chars = longest_gov + 3;
    let cores_per_line = (TERM_LEN - TABLE_LEGEND_LEN) / per_core_chars;

    // generate lines of core descriptions

    println!("Current Settings:");

    for cs in cores.chunks(cores_per_line) {
        let mut creline: String = "Core                   ".into();
        let mut minline: String = "Min CPU/Current [GHz]  ".into();
        let mut maxline: String = "Max CPU/Current [GHz]  ".into();
        let mut govline: String = "Current Governor       ".into();

        for core in cs.iter() {
            let mut pad_to = creline.len() + per_core_chars;
            creline.push_str(&format!(" {}", core.num()));
            minline.push_str(&format!(
                " {:03.3}/{:03.3}",
                f64::from(core.cpu_min()) / 1e6,
                f64::from(core.curr_min()) / 1e6
            ));
            maxline.push_str(&format!(
                " {:03.3}/{:03.3}",
                f64::from(core.cpu_max()) / 1e6,
                f64::from(core.curr_max()) / 1e6
            ));
            govline.push_str(&format!(" {}", core.curr_gov()));
            for line in [&mut creline, &mut minline, &mut maxline, &mut govline].iter_mut() {
                while line.len() < pad_to {
                    line.push(' ');
                }
                // not a normal vertical line but box drawing character U+2502
                // https://en.wikipedia.org/wiki/Box-drawing_character
                line.push('│');
            }
        }

        println!("{}", creline);
        println!("{}", minline);
        println!("{}", maxline);
        println!("{}", govline);

        let mut divider = String::with_capacity(TERM_LEN);
        (0..creline.len() - 8).for_each(|i| {
            if i < TABLE_LEGEND_LEN {
                divider.push_str(" ");
            } else {
                // not a normal dash but box drawing character U+2500
                // also longer than normal
                divider.push_str("─");
            }
        });
        println!("{}", divider);
    }

    // display all available governors
    //OPT: maybe use color coding so we can show which core supports which (most likely they are not going to differ... so why bother with the terminal color crate)
    let available_governors = cores.iter().fold(HashSet::new(), |mut govs, c| {
        c.available_govs().iter().for_each(|g| {
            let _ = govs.insert(g);
        });
        govs
    });

    let av_govs = available_governors.iter().fold(
        "* Available Governors   ".to_string(),
        |mut av_govs, gov| {
            av_govs.push_str(&format!("{} ", gov));
            av_govs
        },
    );

    println!("{}", av_govs);
    // all good? exit with 0
}

fn print_json(cores: &[Core]) {
    let s = serde_json::to_string_pretty(&cores).expect("Serialisation failed");
    try_or_exit!(stdout().write(s.as_ref()), "Failed to write json to stdout");
}

fn show(json: bool) {
    let cores = discover_cores();

    if json {
        print_json(&cores);
    } else {
        print_pretty(&cores);
    }

    exit(0);
}

fn main() {
    let settings = Mdcr::from_args();
    debug!("Args provided: {:#?}", settings);

    match settings {
        Mdcr::Set(c) => set(c),
        Mdcr::Powersave => powersave(),
        Mdcr::Performance => performance(),
        Mdcr::Show { json } => show(json),
    };
}
