use std::{env, fs, process};
use std::path::PathBuf;
use std::io;
use serde::{Deserialize, Serialize};

const RPC_URL: &str = "https://rpc.pulsechain.com";
const HEX_CONTRACT: &str = "0x2b591e99afE9f32eAA6214f7B7629768c40Eeb39";
const DEXSCREENER_URL: &str = "https://api.dexscreener.com/latest/dex/tokens/0x2b591e99afE9f32eAA6214f7B7629768c40Eeb39";

struct LiveData {
    price_pulsechain: f64,
    tshare_price_pulsechain: f64,
    tshare_rate_hex_pulsechain: f64,
    payout_per_tshare_pulsechain: f64,
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

    let live_data = match fetch_live_data() {
        Ok(data) => data,
        Err(e) => {
            println!("Error fetching live data: {}", e);
            return;
        }
    };

    let tshares_payout = live_data.payout_per_tshare_pulsechain * tshares;
    let tshares_value = live_data.tshare_price_pulsechain * tshares;

    let current_data = SavedData {
        hex_price: live_data.price_pulsechain,
        tshare_price: live_data.tshare_price_pulsechain,
        tshare_rate: live_data.tshare_rate_hex_pulsechain,
        tshare_payout: tshares_payout,
        tshare_value: tshares_value,
        tshares,
    };

    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(home).join("hexfetch-cli");
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
        display_data(&live_data, tshares_payout, tshares_value, tshares);
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

fn call_rpc(method: &str, params: serde_json::Value) -> Result<String, String> {
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    });
    
    let resp = ureq::post(RPC_URL)
        .set("Content-Type", "application/json")
        .send_json(&payload)
        .map_err(|e| e.to_string())?;
        
    let val: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
    
    if let Some(err) = val.get("error") {
        return Err(format!("RPC Error: {}", err));
    }
    
    val["result"].as_str().map(|s| s.to_string()).ok_or_else(|| "Missing result in RPC response".to_string())
}

fn fetch_live_data() -> Result<LiveData, String> {
    // 1. Fetch DEX Price
    let resp = ureq::get(DEXSCREENER_URL)
        .call()
        .map_err(|e| e.to_string())?;
        
    let dex_resp: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
    
    let price = dex_resp["pairs"][0]["priceUsd"]
        .as_f64()
        .or_else(|| dex_resp["pairs"][0]["priceUsd"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0);

    // 2. Fetch Globals
    let globals_data = serde_json::json!([{"to": HEX_CONTRACT, "data": "0xc3124525"}, "latest"]);
    let globals_hex = call_rpc("eth_call", globals_data)?;
    let g_str = globals_hex.strip_prefix("0x").unwrap_or(&globals_hex);

    let mut tshare_rate = 0.0;
    let mut daily_data_count: u128 = 0;

    if g_str.len() >= 320 {
        let share_rate = u128::from_str_radix(&g_str[128..192], 16).unwrap_or(0);
        daily_data_count = u128::from_str_radix(&g_str[256..320], 16).unwrap_or(0);

        if share_rate > 0 {
            tshare_rate = share_rate as f64 / 10.0;
        }
    }

    // 3. Fetch Daily Data
    let mut payout_per_tshare = 0.0;
    if daily_data_count > 0 {
        let day_to_query = daily_data_count - 1;
        let day_padded = format!("0x90de6871{:064x}", day_to_query); 
        let daily_data = serde_json::json!([{"to": HEX_CONTRACT, "data": day_padded}, "latest"]);
        
        if let Ok(daily_hex) = call_rpc("eth_call", daily_data) {
            let d_str = daily_hex.strip_prefix("0x").unwrap_or(&daily_hex);
            if d_str.len() >= 192 {
                let day_payout = u128::from_str_radix(&d_str[0..64], 16).unwrap_or(0);
                let day_shares = u128::from_str_radix(&d_str[64..128], 16).unwrap_or(0);
                
                if day_shares > 0 {
                    // Multiplier 10000.0 accounts for the 4-decimal scaling of T-Shares in the Hex contract
                    payout_per_tshare = (day_payout as f64 / day_shares as f64) * 10000.0;
                }
            }
        }
    }

    Ok(LiveData {
        price_pulsechain: price,
        tshare_price_pulsechain: tshare_rate * price,
        tshare_rate_hex_pulsechain: tshare_rate,
        payout_per_tshare_pulsechain: payout_per_tshare,
    })
}

fn display_data(live_data: &LiveData, tshares_payout: f64, tshares_value: f64, tshares: f64) {
    println!("{:<14} : {:3.6} $", "HEX Price", live_data.price_pulsechain);
    println!("{:<14} : {:3.2} $", "T-Share Price", live_data.tshare_price_pulsechain);
    println!("{:<14} : {:3.1} HEX", "T-Share Rate", live_data.tshare_rate_hex_pulsechain);
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

    let mut has_changes = false;
    for &(_, cv, sv, _, _, _) in &fields {
        if cv != sv {
            has_changes = true;
            break;
        }
    }

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
