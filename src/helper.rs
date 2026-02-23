use aidoku::{
	alloc::{String, Vec},
	imports::html::Element,
	Chapter, Manga, Viewer,
};

/// Map genre name (Chinese) to the Webtoons URL slug.
pub fn genre_name_to_slug(name: &str) -> &'static str {
	match name {
		"愛情" => "romance",
		"歐式宮廷" => "western_palace",
		"影視化" => "adaptation",
		"校園" => "school",
		"台灣原創作品" => "local",
		"奇幻冒險" => "fantasy",
		"驚悚" => "thriller",
		"恐怖" => "horror",
		"武俠" => "martial_arts",
		"LGBTQ+" => "bl_gl",
		"大人系" => "romance_m",
		"劇情" => "drama",
		"動作" => "action",
		"生活/日常" => "slice_of_life",
		"搞笑" => "comedy",
		"穿越/轉生" => "time_slip",
		"現代/職場" => "city_office",
		"懸疑推理" => "mystery",
		"療癒/萌系" => "heartwarming",
		"少年" => "shonen",
		"古代宮廷" => "eastern_palace",
		"小說" => "web_novel",
		_ => "romance",
	}
}

/// Extract `title_no` from a Webtoons URL.
pub fn extract_title_no(url: &str) -> Option<String> {
	let pos = url.find("title_no=")?;
	let start = pos + 9;
	let rest = &url[start..];
	let end = rest.find('&').unwrap_or(rest.len());
	Some(String::from(&rest[..end]))
}

/// Extract `episode_no` from a Webtoons viewer URL.
pub fn extract_episode_no(url: &str) -> Option<String> {
	let pos = url.find("episode_no=")?;
	let start = pos + 11;
	let rest = &url[start..];
	let end = rest.find('&').unwrap_or(rest.len());
	Some(String::from(&rest[..end]))
}

/// Parse a manga item from listing/genre/search pages.
///
/// Expected HTML structure:
/// ```html
/// <a href="...?title_no=2089" class="link" data-title-no="2089">
///   <div class="image_wrap"><img src="..." /></div>
///   <div class="info_text">
///     <div class="genre">奇幻冒險</div>
///     <strong class="title">全知讀者視角</strong>
///     <div class="author">Author Name</div>
///   </div>
/// </a>
/// ```
pub fn parse_manga_item(item: &Element) -> Option<Manga> {
	let href = item.attr("href")?;
	let title_no = item
		.attr("data-title-no")
		.or_else(|| extract_title_no(&href))?;

	// Title: strong.title
	let title = item
		.select_first("strong.title")
		.and_then(|el: Element| el.text())
		.unwrap_or_default();

	if title.is_empty() {
		return None;
	}

	// Cover image: div.image_wrap img
	let cover = item
		.select_first(".image_wrap img")
		.and_then(|el: Element| el.attr("src"));

	// Author: div.author (may not be present on originals pages)
	let mut manga = Manga {
		key: title_no,
		title,
		cover,
		url: Some(href),
		viewer: Viewer::Webtoon,
		..Default::default()
	};

	if let Some(author_el) = item.select_first(".author") {
		if let Some(author_text) = author_el.text() {
			let authors: Vec<String> = author_text
				.split('/')
				.map(|s: &str| String::from(s.trim()))
				.filter(|s: &String| !s.is_empty())
				.collect();
			if !authors.is_empty() {
				manga.authors = Some(authors);
			}
		}
	}

	// Genre tag (shown on originals pages where author spot has genre)
	if let Some(genre_el) = item.select_first(".genre") {
		if let Some(genre_text) = genre_el.text() {
			manga.tags = Some(aidoku::alloc::vec![genre_text]);
		}
	}

	Some(manga)
}

/// Parse a chapter item from the manga detail page.
///
/// Expected HTML structure:
/// ```html
/// <li>
///   <a href=".../viewer?title_no=2089&episode_no=1">
///     <img src="..." />
///     <span class="subj"><span>1. Title</span></span>
///     <span class="date">2026年2月22日</span>
///   </a>
/// </li>
/// ```
pub fn parse_chapter_item(item: &Element) -> Option<Chapter> {
	let href = item.attr("href")?;

	let episode_no = extract_episode_no(&href);

	// Title: span.subj span or .subj span
	let title = item
		.select_first(".subj span")
		.and_then(|el: Element| el.text())
		.or_else(|| {
			item.select_first(".subj")
				.and_then(|el: Element| el.text())
		});

	let chapter_num = episode_no
		.as_ref()
		.and_then(|ep: &String| ep.parse::<f32>().ok());

	let date_str = item
		.select_first(".date")
		.and_then(|el: Element| el.text());

	let date_uploaded = date_str.and_then(|d: String| parse_date(&d));

	let thumbnail = item
		.select_first("img")
		.and_then(|el: Element| el.attr("src"));

	// Check if chapter is locked (paid/premium content)
	let locked = item.select_first(".ico_lock").is_some()
		|| item.select_first(".ico_bgm").is_some();

	Some(Chapter {
		key: href.clone(),
		title,
		chapter_number: chapter_num,
		date_uploaded,
		url: Some(href),
		thumbnail,
		locked,
		..Default::default()
	})
}

/// Parse a Webtoons date string like "2026年2月22日" into a Unix timestamp.
pub fn parse_date(date_str: &str) -> Option<i64> {
	let mut year: i64 = 0;
	let mut month: i64 = 0;
	let mut day: i64 = 0;

	let mut num_buf = String::new();
	let mut state = 0;

	for ch in date_str.chars() {
		if ch.is_ascii_digit() {
			num_buf.push(ch);
		} else if ch == '年' && state == 0 {
			year = num_buf.parse().unwrap_or(0);
			num_buf.clear();
			state = 1;
		} else if ch == '月' && state == 1 {
			month = num_buf.parse().unwrap_or(0);
			num_buf.clear();
			state = 2;
		} else if ch == '日' && state == 2 {
			day = num_buf.parse().unwrap_or(0);
			num_buf.clear();
		}
	}

	if year == 0 || month == 0 || day == 0 {
		return None;
	}

	let mut days: i64 = 0;
	for y in 1970..year {
		days += if is_leap_year(y) { 366 } else { 365 };
	}

	let month_days: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
	for m in 1..month {
		let idx = (m - 1) as usize;
		if idx < 12 {
			days += month_days[idx];
			if m == 2 && is_leap_year(year) {
				days += 1;
			}
		}
	}

	days += day - 1;

	Some(days * 86400)
}

fn is_leap_year(year: i64) -> bool {
	(year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
