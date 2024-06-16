/*!
 * Print information about an OmniOS machine.
 *
 * Author: Dave Eddy <dave@daveeddy.com>
 * Date: June 15, 2024
 * License: MIT
 */

use std::collections::HashMap;
use std::env;
use std::fs;
use std::time::SystemTime;

use anyhow::{Context, Result};
use indexmap::IndexMap;

mod util;

const FENIX: &str = include_str!("../files/fenix.txt");
const OMNIOS: &str = include_str!("../files/omnios.txt");

fn get_hostname() -> Result<String> {
    let name =
        nix::unistd::gethostname()?.into_string().expect("invalid hostname");
    Ok(name)
}

fn get_user() -> Result<String> {
    let user = env::var("USER").context("failed to get user")?;
    Ok(user)
}

fn get_os() -> Result<String> {
    let data = fs::read_to_string("/etc/release")?;
    let line = data.lines().next().context("expected at least 1 line")?;

    let s = line.trim().into();

    Ok(s)
}

fn get_zonename() -> Result<String> {
    let name = zonename::getzonename()?;
    Ok(name)
}

fn get_kernel() -> Result<String> {
    run! { "uname -v" }
}

fn get_cpu() -> Result<String> {
    let output = run! { "kstat -p cpu_info:::brand" }?;
    let lines = output.lines();

    let mut counts = HashMap::new();
    for line in lines {
        let spl: Vec<_> = line.split('\t').collect();
        let brand = spl[1];
        *counts.entry(brand).or_insert(0) += 1;
    }

    let mut brands = vec![];
    for (brand, count) in counts {
        let s = format!("{} x {}", count, brand);
        brands.push(s);
    }
    let s = brands.join(", ");

    Ok(s)
}

fn get_memory() -> Result<String> {
    let output = run! { "lgrpinfo -m" }?;
    let lines: Vec<_> = output.lines().collect();

    let spl: Vec<_> = lines[1].split(':').collect();
    let s = spl[1].trim().into();

    Ok(s)
}

fn get_uptime() -> Result<String> {
    let output = run! { "kstat -p unix:0:system_misc:boot_time" }?;
    let spl: Vec<_> = output.split('\t').collect();
    let booted: u64 = spl[1].parse()?;

    let now =
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs();

    let uptime_secs = now - booted;
    let uptime_days = uptime_secs / 60 / 60 / 24;

    let s = format!("up {} days", uptime_days);

    Ok(s)
}

fn get_smf() -> Result<String> {
    let output = run! { "svcs -H -o state" }?;

    let mut counts = HashMap::new();
    for line in output.lines() {
        *counts.entry(line).or_insert(0) += 1;
    }

    let online = counts.get("online").unwrap_or(&0);
    let s = format!("{} svcs online", online);

    Ok(s)
}

fn get_bootenvironment() -> Result<String> {
    let output = run! { "beadm list -H" }?;

    let mut next = None;
    let mut current = None;
    for line in output.lines() {
        let spl: Vec<_> = line.split(';').collect();
        let name = spl[0];
        let flags = spl[2];

        if flags.contains('R') {
            assert!(next.is_none());
            next = Some(name.to_string());
        }
        if flags.contains('N') {
            assert!(current.is_none());
            current = Some(name.to_string());
        }
    }

    let next = next.context("couldn't find next be")?;
    let current = current.context("couldn't find current be")?;

    let s = if next == current {
        current
    } else {
        format!("{} (staged {})", current, next)
    };

    Ok(s)
}

fn get_zones() -> Result<String> {
    let running = run! { "zoneadm list -n" }?.lines().count();
    let total = run! { "zoneadm list -cn" }?.lines().count();

    let s = format!("{} running ({} total)", running, total);

    Ok(s)
}

fn get_zpools() -> Result<String> {
    let output = run! { "zpool list -Ho name,cap,alloc,size" }?;

    let mut zpools = vec![];
    for line in output.lines() {
        let spl: Vec<_> = line.split_whitespace().collect();
        let name = spl[0].to_string();
        let _used = spl[1].to_string();
        let alloc = spl[2].to_string();
        let size = spl[3].to_string();

        zpools.push(format!("{} {}/{}", name, alloc, size));
    }

    let s = zpools.join(", ");

    Ok(s)
}

fn main() -> Result<()> {
    // gather data
    let mut data = IndexMap::new();
    let user = get_user()?;
    let hostname = get_hostname()?;
    data.insert("OS", get_os()?);
    data.insert("Kernel", get_kernel()?);
    data.insert("Zonename", get_zonename()?);
    data.insert("Boot Env", get_bootenvironment()?);
    data.insert("CPU", get_cpu()?);
    data.insert("Uptime", get_uptime()?);
    data.insert("Memory", get_memory()?);
    data.insert("SMF", get_smf()?);
    data.insert("Zones", get_zones()?);
    data.insert("ZFS", get_zpools()?);

    // format output - "output" here will contain all of the data that goes to
    // the right of the fenix logo
    let mut output = vec![];

    // first queue up the omnios logo
    for line in OMNIOS.lines() {
        output.push(line.to_string());
    }
    output.push("".to_string());

    // next format the user and hostname
    output.push(format!("$(c1){}$(c2)@$(c1){}", user, hostname));
    let num_dashes = user.len() + 1 + hostname.len();
    output.push(format!("$(c2){}", "-".repeat(num_dashes)));
    output.push("".to_string());

    // finally print the gathered data
    for (key, value) in data {
        output.push(format!("$(c1){}:$(c0) {}", key, value));
    }

    let fenix_lines: Vec<_> = FENIX.lines().collect();

    // generate output by prefixing the gathered data with the fenix logo
    println!();
    for (i, fenix_line) in fenix_lines.into_iter().enumerate() {
        let output_line = match output.get(i) {
            Some(s) => s,
            None => "",
        };

        let s = format!("{} {}", fenix_line, output_line);
        println!("{}", util::colorize(&s));
    }
    println!();

    Ok(())
}
