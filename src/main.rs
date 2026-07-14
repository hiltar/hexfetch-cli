use std::{env, fs, process};
use std::path::PathBuf;
use std::io;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
struct ApiResp {
    #[serde(rename = "price_Pulsechain")]
    hex_price: f64,
    #[serde(rename = "tsharePrice_Pulsechain")]
    tshare_price: f64,
    #[serde(rename = "tshareRateHEX_Pulsechain")]
    tshare_rate_hex: f64,
    #[serde(rename = "payoutPerTshare_Pulsechain")]
    tshare_payout: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SavedData {
    #[serde(rename = "HEX Price")]
    hex_price: f64,
    #[serde(rename = "T-Share Price")]
    tshare_price: f64,
    #[serde(rename = "T-Share Rate")]
    tshare_rate: f64,
    #[serde(rename = "T-Share Payout")]
    tshare_payout: f64,
    #[serde(rename = "T-Share Value")]
    tshare_value: f64,
    #[serde(rename = "T-Shares")]
    tshares: f64,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let tshares = if args.len() > 1 {
        args[1].parse::<f64>().unwrap_or_else(|e| {
            println!("Invalid number of TShares: {}", e);
            process::exit(1);
        })
    } else {
        1.0
    };

    let api_resp = match fetch_api_data() {
        Ok(resp) => resp,
        Err(e) => {
            println!("Error fetching API data: {}", e);
            return;
        }
    };

    let tshares_payout = api_resp.tshare_payout * tshares;
    let tshares_value = api_resp.tshare_price * tshares;

    let current_data = SavedData {
        hex_price: api_resp.hex_price,
        tshare_price: api_resp.tshare_price,
        tshare_rate: api_resp.tshare_rate_hex,
        tshare_payout: tshares_payout,
        tshare_value: tshares_value,
        tshares,
    };

    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(home).join("hexfetch");
    let _ = fs::create_dir_all(&dir);
    let filename = dir.join("saved_hexdata.json");

    let saved_data = match load_from_file(&filename) {
        Ok(data) => data,
        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                println!("{}", e);
            }
            current_data.clone()
        }
    };

    let has_changes = compare_data(&current_data, &saved_data);

    if !has_changes {
        display_data(&api_resp, tshares_payout, tshares_value, tshares);
    }

    if let Err(e) = save_to_file(&filename, &current_data) {
        println!("{}", e);
    }
}

fn save_to_file(filename: &PathBuf, data: &SavedData) -> io::Result<()> {
    let file = serde_json::to_string_pretty(data)?;
    fs::write(filename, file)
}

fn load_from_file(filename: &PathBuf) -> io::Result<SavedData> {
    let file_content = fs::read_to_string(filename)?;
    let data: SavedData = serde_json::from_str(&file_content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(data)
}

fn fetch_api_data() -> Result<ApiResp, String> {
    let resp = ureq::get("https://hexdailystats.com/livedata")
        .set("Accept", "application/json")
        .call()
        .map_err(|e| format!("HTTP error: {}", e))?;

    if resp.status() != 200 {
        return Err(format!("Received non-OK HTTP status: {}", resp.status()));
    }

    let api_resp: ApiResp = resp.into_json().map_err(|e| format!("JSON parse error: {}", e))?;
    Ok(api_resp)
}

fn display_data(api_resp: &ApiResp, tshares_payout: f64, tshares_value: f64, tshares: f64) {
    println!("{:<14} : {:3.6} $", "HEX Price", api_resp.hex_price);
    println!("{:<14} : {:3.2} $", "T-Share Price", api_resp.tshare_price);
    println!("{:<14} : {:3.1} HEX", "T-Share Rate", api_resp.tshare_rate_hex);
    println!("{:<14} : {:3.3} HEX", "T-Share Payout", tshares_payout);
    println!("{:<14} : {:3.2} $", "T-Share Value", tshares_value);
    println!("{:<14} : {:3.2}", "T-Shares", tshares);
}

fn compare_data(current: &SavedData, saved: &SavedData) -> bool {
    let fields = [
        ("HEX Price", current.hex_price, saved.hex_price, 6, "$", 6),
        ("T-Share Price", current.tshare_price, saved.tshare_price, 2, "$", 6),
        ("T-Share Rate", current.tshare_rate, saved.tshare_rate, 1, "HEX", 6),
        ("T-Share Payout", current.tshare_payout, saved.tshare_payout, 3, "HEX", 6),
        ("T-Share Value", current.tshare_value, saved.tshare_value, 2, "$", 6),
        ("T-Shares", current.tshares, saved.tshares, 2, "", 2),
    ];

    // First pass: check for any changes to avoid allocating a Vec unnecessarily
    let mut has_changes = false;
    for &(_, cv, sv, _, _, _) in &fields {
        if cv != sv {
            has_changes = true;
            break;
        }
    }

    // Second pass: if changes exist, calculate and print the formatted output
    if has_changes {
        for (key, cv, sv, prec, suffix, diff_prec) in fields {
            let diff = cv - sv;
            let has_diff = cv != sv;
            
            let val_str = if suffix.is_empty() {
                format!("{:3.prec$}", cv, prec = prec)
            } else {
                format!("{:3.prec$} {}", cv, suffix, prec = prec)
            };
            
            if has_diff {
                let diff_str = format!("{:+.prec$}", diff, prec = diff_prec);
                println!("{:<14} : {} ({})", key, val_str, diff_str);
            } else {
                println!("{:<14} : {}", key, val_str);
            }
        }
    }
    
    has_changes
}
