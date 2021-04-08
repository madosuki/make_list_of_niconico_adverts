use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

use serde::Deserialize;
use regex::Regex;
use chrono::DateTime;
use clap::{Arg, App};

// struct OptionError {
//     e: String
// }

// impl fmt::Display for OptionError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "Error: {}", self.0)
//     }
// }

// impl std::error::Error for OptionError {
//     fn description(&self) -> &str { &self.e }
// }


enum Mode {
    Normal,
    WithCount
}

#[derive(Deserialize, Debug)]
struct Thumb {
    video_id: String,
    title: String,
    description: String,
    thumbnail_url: String,
    first_retrieve: String,
    length: String,
    movie_type: String,
    size_high: i64,
    size_low: i64,
    view_counter: i64,
    comment_num: i64,
    mylist_counter: i64,
    last_res_body: String,
    watch_url: String,
    thumb_type: String,
    embeddable: i64,
    no_live_play: i64,
    genre: String,
    user_id: i64,
    user_nickname: String,
    user_icon_url: String
}

#[derive(Deserialize, Debug)]
struct VideoInfo {
    status: String,
    thumb: Thumb
}



#[derive(Deserialize, Debug)]
struct Status {
    status: i32
}

#[derive(Deserialize, Debug)]
struct Data {
    count: i64,
    
    #[serde(alias = "serverTime")]
    server_time: i64,
    
    histories: Vec<Histories>
}

#[derive(Deserialize, Debug)]
struct Histories {
    #[serde(alias = "advertiserName")]
    advertiser_name: String,

    #[serde(alias = "nicoadId")]
    nico_ad_id: i64,

    #[serde(alias = "userId")]
    user_id: Option<i64>,

    #[serde(alias = "adPoint")]
    ad_point: i64,

    contribution: i64,

    #[serde(alias = "startedAt")]
    started_at: i64,

    #[serde(alias = "endedAt")]
    end_at: i64,

    message: Option<String>
}

#[derive(Deserialize, Debug)]
struct NicoNicoAdvert {
    meta: Status,
    data: Data
}

struct DownloadData {
    original: Vec<String>,
    with_count: HashMap<String, i64>
}


enum _IsRenewal {
    _Before,
    _After
}


async fn check_before_2017_12_12_or_after(video_id: &str) -> Result<_IsRenewal, Box<dyn std::error::Error>> {
    let detail_query = format!("https://ext.nicovideo.jp/api/getthumbinfo/{}", video_id);
    let response = reqwest::get(detail_query).await?.text().await?;
    
    let xml: VideoInfo = serde_xml_rs::from_str(&response).unwrap();

    let first_retrieve = xml.thumb.first_retrieve;

    let _target = DateTime::parse_from_rfc3339(&first_retrieve).unwrap();
    let _boundary_date = DateTime::parse_from_rfc3339(&"2017-12-13T00:00:00+09:00").unwrap();
    
    Ok(if _target < _boundary_date { _IsRenewal::_Before } else { _IsRenewal::_After })
}

async fn create_list_from_csv(video_id: &str) -> Result<DownloadData, Box<dyn std::error::Error>> {
    let mut result: DownloadData = DownloadData {original: vec!(), with_count: HashMap::new()};
    
    let query = format!("https://secure-dcdn.cdn.nimg.jp/nicoad/res/old-video-comments/{}.csv", video_id);
    let response = reqwest::get(query).await?;
    let status = response.status();
    if status != 200 {
        return Err("status code is not 200 in create_list_from_csv".into())
    }

    
    let text = response.text().await?;
    let splited_from_newline = text.split("\n");
    
    let set_data = |x: &str| {
        let splited_from_camma: Vec<&str> = x.split(",").collect();
        let key: String = splited_from_camma[0].to_owned();

        if key.len() > 3 {
            let len = key.len();
            let final_key = &key.clone()[1..len - 1].to_string();
            
            *result.with_count.entry(final_key.clone()).or_insert(0) += 1;
            result.original.push(final_key.to_string());
            
        }
    };

    splited_from_newline.for_each(set_data);

    Ok(result)
}

async fn create_list_from_json(video_id: &str) -> Result<DownloadData, Box<dyn std::error::Error>> {
    let mut result: DownloadData = DownloadData {original: vec!(), with_count: HashMap::new()};
    
    let mut i = 0;
    let page = 128;
    
    loop {
        let query = format!("https://api.nicoad.nicovideo.jp/v1/contents/video/{}/histories?offset={}&limit={}", video_id, i, page);

        let response = reqwest::get(query).await?.json::<NicoNicoAdvert>().await?;

        let data = response.data;
        let len = &data.histories.len();

        
        for i in data.histories {

            let key = i.advertiser_name;
            
            *result.with_count.entry(key.clone()).or_insert(0) += 1;
            result.original.push(key);
            
        }

        if len < &1 {
            break;
        }
        
        i = i + len;
    }

    Ok(result)
}

async fn before_process(video_id: &str) -> Result<Option<DownloadData>, Box<dyn std::error::Error>> {

    let a = create_list_from_json(&video_id).await;
    let mut is_got_json = false;
    match a {
        Ok(_) => {
            is_got_json = true;
        },
        _ => {
            
        }
    }

    let b = create_list_from_csv(&video_id).await;
    let mut is_got_csv = false;
    match b {
        Ok(_) => {
            is_got_csv = true;
        },
        _ => {
            
        }
    }

    match (is_got_csv, is_got_json) {
        (false, true) => {
            Ok(Some(a.unwrap()))
        },
        (true, false) => {
            Ok(Some(b.unwrap()))  
        },
        (true, true) => {
            let mut result: DownloadData = DownloadData {original: vec!(), with_count: HashMap::new()};
            let left = a.unwrap();
            for i in left.with_count {
                result.with_count.insert(i.0, i.1);
            }

            let right = b.unwrap();
            for i in right.with_count {
                *result.with_count.entry(i.0.to_string()).or_insert(0) += i.1;
            }

            result.original = left.original.into_iter().chain(right.original).collect();

            
            Ok(Some(result))
        },
        _ => {
            Ok(None)
        },
    }
}

fn shape_text(data: DownloadData, mode: Mode, width: i32) -> String {
    
    match mode {
        Mode::WithCount => {
            let mut s: String = "".to_owned();
            let mut count = 0;
            
            for i in data.with_count {
                if count > 0 {
                    s = format!("{} {}x{}", s, i.0, i.1);
                } else {
                    s = format!("{}{}x{}", s, i.0, i.1);
                }

                count += 1;

                if count == width {
                    s = s + "\n";
                    count = 0;
                }
            }

            s
        },
        Mode::Normal => {

            let mut s: String = "".to_owned();
            let mut count = 0;
            for i in data.original {
                if count > 0 {
                    s = format!("{} {}", s, i);
                } else {
                    s = format!("{}{}", s, i);
                }

                count += 1;

                if count == width {
                    s = s + "\n";
                    count = 0;
                }
            }
            
            s
        }
    }
}


fn write_to_file(video_id: &str, s: &str) -> std::io::Result<()> {
    
    let mut file = File::create(format!("{}_list.txt", video_id))?;

    file.write_all(s.as_bytes())
}


async fn get_list(url: &str, width: i32) -> Result<(), Box<dyn std::error::Error>> {

    let video_id: String;
    let re = Regex::new(r"https://www.nicovideo.jp/watch/(sm[0-9]+)$").unwrap();
    let capture = re.captures(url);
    match capture {
        Some(x) => {
            video_id = x[1].to_string();
        }
        _ => {
            return Err("invalid url in get_list".into())
        }
    }

    
    match check_before_2017_12_12_or_after(&video_id).await? {
        _IsRenewal::_Before => {
            println!("before");
            let c = before_process(&video_id).await?;
            if c.is_some() {
                let result = c.unwrap();
                let final_text = shape_text(result, Mode::WithCount, width);
                let r = write_to_file(&video_id, &final_text);
                if r.is_err() {
                    Err("write error in before process in match".into())
                } else {
                    Ok(())
                }
            } else {
                Err("detect error in before_process".into())
            }
        },
        _IsRenewal::_After => {
            let a = create_list_from_json(&video_id).await?;
            let final_text = shape_text(a, Mode::Normal, width);
            let r = write_to_file(&video_id, &final_text);
            if r.is_err() {
                Err("write error in after process".into())
            } else {
                Ok(())
            }

        }
    }

}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let _url = "https://www.nicovideo.jp/watch/sm38531871";
    // let _url = "https://www.nicovideo.jp/watch/sm25597642";
    // let _url = "https://www.nicovideo.jp/watch/sm31881208";

    
    let matches = App::new("make list of niconico_adverts")
        .arg(Arg::with_name("url")
             .short("u")
             .long("url")
             .takes_value(true)
             .help("video url.")
        )
        .arg(Arg::with_name("width")
             .short("w")
             .long("width")
             .takes_value(true)
             .help("number of name in per line. this param is optional. default value of 3."))
        .get_matches();

    let _url = matches.value_of("url").unwrap_or("");
    let tmp_width = matches.value_of("width").unwrap_or("");

    if _url.len() == 0 {
        return Err("require url".into())
    }

    let width: i32;
    if tmp_width.len() > 0 {
        width = tmp_width.parse().unwrap_or(3);
    } else {
        width = 3;
    }

    get_list(_url, width).await?;
    
    Ok(())
}
