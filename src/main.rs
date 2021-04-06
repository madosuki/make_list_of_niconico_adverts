use std::collections::HashMap;
use serde::{Deserialize};
use regex::Regex;
use chrono::{DateTime, NaiveDateTime};


#[derive(Deserialize, Debug)]
struct NicoNicoAdvert {
    meta: Status,
    data: Data
}

#[derive(Deserialize, Debug)]
struct Status {
    status: i32
}


#[derive(Deserialize, Debug)]
struct Tag {
    tag: String
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

async fn check_before_2017_12_12_or_after(video_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let detail_query = format!("https://ext.nicovideo.jp/api/getthumbinfo/{}", video_id);
    let response = reqwest::get(detail_query).await?.text().await?;
    
    let xml: VideoInfo = serde_xml_rs::from_str(&response).unwrap();

    let first_retrieve = xml.thumb.first_retrieve;
    println!("{}", first_retrieve);
    let _target = DateTime::parse_from_rfc3339(&first_retrieve).unwrap();
    let _boundary_date = DateTime::parse_from_rfc3339(&"2017-12-13T00:00:00+09:00").unwrap();

    Ok(if _target < _boundary_date { false } else { true })
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _url = "https://www.nicovideo.jp/watch/sm38531871";

    let video_id: String;
    let re = Regex::new(r"https://www.nicovideo.jp/watch/(sm[0-9]+)$").unwrap();
    let capture = re.captures(_url);
    match capture {
        Some(x) => {
            video_id = x[1].to_string();
        }
        _ => {
            println!("invalid url");
            panic!("")
        }
    }

    let check = check_before_2017_12_12_or_after(&video_id).await?;
        

    let mut result: HashMap<String, i64> = HashMap::new();
    
    let mut i = 0;
    let page = 128;
    loop {
        let query = format!("https://api.nicoad.nicovideo.jp/v1/contents/video/{}/histories?offset={}&limit={}", video_id, i, page);

        let response = reqwest::get(query).await?.json::<NicoNicoAdvert>().await?;

        let data = response.data;
        let len = &data.histories.len();

        
        for i in data.histories {
            
            *result.entry(i.advertiser_name).or_insert(0) += 1;
            
        }

        if len < &1 {
            break;
        }
        
        i = i + len;
    }

    for i in result {
        println!("key: {}, count: {}", i.0, i.1);
    }
    
    
    Ok(())
}
