use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

const MAX_ATTEMPTS: usize = 3;
const RECENT_IN_PROMPT: usize = 12;
const HISTORY_COMPARE: usize = 30;
const DEFAULT_STYLE: &str = "commute";
const DEFAULT_SHARPNESS: &str = "quiet";

const STARTING_POINTS: &[&str] = &[
    "等电梯时每个人都盯着楼层数字",
    "深夜厨房里只剩冰箱的低鸣",
    "下雨通勤时鞋尖先湿了一小块",
    "会议结束后屏幕还停在共享画面",
    "付款失败后重新输入验证码",
    "删除聊天记录前手指停了一秒",
    "整理旧物时翻到一张过期票根",
    "手机电量到 1% 时突然变得诚实",
    "凌晨自动售货机的灯比人更清醒",
    "洗衣机停止后房间短暂安静",
    "输入框里删掉一整段没有发出的话",
    "外卖袋里的塑料勺永远多一把",
    "地铁进站前风先替人抵达",
    "电梯镜子里的人都像临时演员",
    "浏览器标签页多到看不见标题",
    "日历提醒弹出时事情已经变形",
    "打包行李时最占地方的是犹豫",
    "耳机断连后世界忽然恢复原价",
    "便利店微波炉倒计时的最后三秒",
    "门锁咔哒一声后房间开始替人保密",
    "电脑风扇突然转响，像某种辩解",
    "闹钟响过一次后清晨变得可疑",
    "排队时前面的人总在最后一刻加东西",
    "备忘录里躺着一堆从未开始的改变",
];

const ANGLES: &[&str] = &[
    "写出一个人如何把小事当成秩序",
    "写出控制感如何在细节里露馅",
    "写出亲密关系里的轻微错位",
    "写出效率背后隐藏的疲惫",
    "写出人对确定性的迷信",
    "写出一个动作里的自我欺骗",
    "写出城市生活里不被承认的孤独",
    "写出时间如何被琐事偷偷拿走",
    "写出体面和狼狈之间的薄边界",
    "写出沉默比表达更费力的时刻",
    "写出某种轻微但准确的自嘲",
    "写出人如何用忙碌绕开真正的问题",
];

const TONES: &[&str] = &[
    "冷一点，但不要刻薄",
    "轻微幽默，但不要段子感",
    "安静，像刚刚想明白又不急着说服别人",
    "有一点刺痛，但不煽情",
    "克制，留一点没说完的余味",
    "平实，像把一个小事实摆正",
    "温柔但不安慰",
    "疏离，像站远半步看自己",
];

const TEXTURES: &[&str] = &[
    "保留一个具体物件",
    "保留一个声音或动作",
    "保留一点时间感",
    "保留轻微的荒诞感",
    "让结尾落在一个具体细节上",
    "让句子像观察，不像结论",
    "少用抽象名词，多用可看见的东西",
    "让人读完后先停一下，而不是立刻点头",
];

pub fn generate(
    recent: &[String],
    style: &str,
    sharpness: &str,
    personal_note: &str,
) -> Result<(String, String)> {
    let path = enriched_path();
    let mut failures = Vec::new();
    let mut last_candidate: Option<(String, String)> = None;
    let style = normalize_style(style);
    let sharpness = normalize_sharpness(sharpness);
    let personal_note = personal_note.trim();

    for attempt in 0..MAX_ATTEMPTS {
        let prompt = build_prompt(recent, &failures, attempt, style, sharpness, personal_note);

        if let Some(text) = try_claude(&path, &prompt) {
            last_candidate = Some((text.clone(), "claude".into()));
            match quality_issue(&text, recent) {
                None => return Ok((text, "claude".into())),
                Some(reason) => failures.push(format!("{text}（{reason}）")),
            }
        }

        if let Some(text) = try_codex(&path, &prompt) {
            last_candidate = Some((text.clone(), "codex".into()));
            match quality_issue(&text, recent) {
                None => return Ok((text, "codex".into())),
                Some(reason) => failures.push(format!("{text}（{reason}）")),
            }
        }
    }

    if let Some(candidate) = last_candidate {
        return Ok(candidate);
    }

    Err(anyhow!(
        "未找到可用的 claude 或 codex CLI——请确认已安装并位于 PATH"
    ))
}

pub fn normalize_style(style: &str) -> &'static str {
    match style {
        "commute" => "commute",
        "night" => "night",
        "work" => "work",
        "tender" => "tender",
        "weekend" => "weekend",
        _ => DEFAULT_STYLE,
    }
}

pub fn normalize_sharpness(sharpness: &str) -> &'static str {
    match sharpness {
        "soft" => "soft",
        "quiet" => "quiet",
        "sharp" => "sharp",
        _ => DEFAULT_SHARPNESS,
    }
}

fn try_claude(path: &OsString, prompt: &str) -> Option<String> {
    let out = Command::new("claude")
        .env("PATH", path)
        .args(["-p", prompt])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = clean(String::from_utf8_lossy(&out.stdout).to_string());
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn try_codex(path: &OsString, prompt: &str) -> Option<String> {
    let out = Command::new("codex")
        .env("PATH", path)
        .args(["exec", prompt])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = clean(String::from_utf8_lossy(&out.stdout).to_string());
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn build_prompt(
    recent: &[String],
    failures: &[String],
    attempt: usize,
    style: &str,
    sharpness: &str,
    personal_note: &str,
) -> String {
    let seed = entropy(attempt);
    let starting_point = pick(STARTING_POINTS, seed, 0x9e37_79b9_7f4a_7c15);
    let angle = pick(ANGLES, seed, 0xc2b2_ae3d_27d4_eb4f);
    let tone = pick(TONES, seed, 0x1656_67b1_9e37_79f9);
    let texture = pick(TEXTURES, seed, 0x85eb_ca6b_27d4_eb2f);
    let style_note = style_note(style);
    let sharpness_note = sharpness_note(sharpness);
    let personal_note = if personal_note.is_empty() {
        "无。不要臆造用户身份，可以从任何真实、普通、微小的日常切口取材。".to_string()
    } else {
        format!(
            "{}。这是用户自己的生活关键词，只当气味和方向，不要逐字堆砌。",
            personal_note.chars().take(80).collect::<String>()
        )
    };
    let recent_block = numbered_block(recent.iter().take(RECENT_IN_PROMPT));
    let failure_block = numbered_block(failures.iter().rev().take(6));

    format!(
        "你在为一个常驻桌面的微型产品 Sotto 写内容。它每次只显示一句话。\n\
这句话应该像用户在日常某个具体瞬间突然看见的观察，不是人生建议，也不是名人名言。\n\n\
可借用的起点，不是命题：\n\
- 起点：{starting_point}\n\
- 观察角度：{angle}\n\
- 情绪温度：{tone}\n\
- 语言质地：{texture}\n\
- 风格包：{style_note}\n\
- 语气强度：{sharpness_note}\n\
- 私人偏好：{personal_note}\n\
- 随机种子：{}\n\n\
输出要求：\n\
1. 只写一句简体中文，24~56 字，单行\n\
2. 上面的起点只用于打破惯性；如果你想到更准确的切口，可以完全不用它\n\
3. 不要为了满足所有素材而显得工整，宁可自由、偏一点、像突然捕捉到的念头\n\
4. 最好带一个具体物件、动作、声音或空间细节，但不要写成完整故事\n\
5. 可以微妙、冷幽默、轻微刺痛，也可以只是安静准确；不要鸡汤、励志、劝告、解释\n\
6. 少用“人生、成长、成熟、努力、热爱、世界、命运”等大词，除非它们非常必要\n\
7. 避免“越是...越是...”“不是...而是...”“所谓...就是...”“真正的...”这类常见格言句式\n\
8. 不要标题、引号、编号、前言、备注；直接输出句子本身\n\n\
最近已经出现过的内容，必须避开相同意象和句式：\n\
{recent_block}\n\n\
刚才被判定太像或太俗的候选，也必须避开：\n\
{failure_block}",
        seed % 100_000
    )
}

fn style_note(style: &str) -> &'static str {
    match style {
        "night" => "深夜自嘲。更安静、更私人，像房间里没关掉的一盏小灯；可以有一点疲惫和清醒。",
        "work" => "上班静音。围绕会议、消息、表格、协作、体面和消耗；不要职场口号。",
        "tender" => {
            "关系微刺痛。写人与人之间的轻微错位、没说出口的话、礼貌里的距离；不要爱情宣言。"
        }
        "weekend" => "周末松弛。写慢下来后的空白、家务、散步、旧物和小小的逃离；不要治愈鸡汤。",
        _ => "通勤冷幽默。围绕路上、等待、拥挤、手机、电梯和城市小故障；冷一点，但不要刻薄。",
    }
}

fn sharpness_note(sharpness: &str) -> &'static str {
    match sharpness {
        "soft" => "更柔和，更像把情绪放低；不要安慰腔。",
        "sharp" => "更锋利，更有微妙刺痛；不要刻薄和冒犯。",
        _ => "克制、留白，像低声说出一个准确观察。",
    }
}

fn clean(raw: String) -> String {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let line = trimmed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !looks_like_meta(line))
        .last()
        .unwrap_or(trimmed);

    let mut text = strip_prefix_noise(line).to_string();
    text = text
        .trim()
        .trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\'' | '“' | '”' | '「' | '」' | '《' | '》' | '`' | ' '
            )
        })
        .to_string();

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn quality_issue(text: &str, recent: &[String]) -> Option<String> {
    let count = text.chars().filter(|c| !c.is_whitespace()).count();
    if count < 16 {
        return Some("过短".into());
    }
    if count > 86 {
        return Some("过长".into());
    }
    if looks_formulaic(text) {
        return Some("句式太像常见格言".into());
    }
    if contains_big_words(text) {
        return Some("抽象大词太多".into());
    }
    for old in recent.iter().take(HISTORY_COMPARE) {
        if old.trim() == text.trim() {
            return Some("和近期内容完全重复".into());
        }
        if too_similar(old, text) {
            return Some("和近期内容过于相似".into());
        }
    }
    None
}

fn looks_formulaic(text: &str) -> bool {
    (text.contains("越是") && text.matches('越').count() >= 2)
        || (text.contains("不是") && text.contains("而是"))
        || text.contains("真正的")
        || text.starts_with("所谓")
        || text.contains("终将")
        || text.contains("总会")
        || text.contains("你要")
        || text.contains("不要害怕")
}

fn contains_big_words(text: &str) -> bool {
    let hits = [
        "人生", "成长", "成熟", "努力", "热爱", "命运", "世界", "内心", "灵魂", "梦想",
    ]
    .iter()
    .filter(|word| text.contains(**word))
    .count();
    hits >= 2
}

fn too_similar(a: &str, b: &str) -> bool {
    let a_chars = meaningful_chars(a);
    let b_chars = meaningful_chars(b);
    let min_len = a_chars.len().min(b_chars.len());
    if min_len < 10 {
        return false;
    }
    let common = a_chars.intersection(&b_chars).count();
    common * 100 >= min_len * 72
}

fn meaningful_chars(text: &str) -> HashSet<char> {
    text.chars()
        .filter(|c| {
            !c.is_whitespace()
                && !c.is_ascii_punctuation()
                && !"，。！？；：、,.!?;:（）()[]【】《》“”\"'".contains(*c)
        })
        .collect()
}

fn entropy(attempt: usize) -> u64 {
    let now = chrono::Local::now();
    let nanos = now
        .timestamp_nanos_opt()
        .unwrap_or_else(|| now.timestamp_millis() * 1_000_000);
    (nanos as u64)
        .wrapping_add((std::process::id() as u64) << 16)
        .wrapping_add((attempt as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15))
        .rotate_left(((attempt * 11) % 63) as u32)
}

fn pick<'a>(items: &'a [&str], seed: u64, salt: u64) -> &'a str {
    let index = seed.wrapping_add(salt).rotate_left(17) as usize % items.len();
    items[index]
}

fn numbered_block<'a>(items: impl Iterator<Item = &'a String>) -> String {
    let lines: Vec<String> = items
        .enumerate()
        .map(|(index, item)| format!("{}. {}", index + 1, item))
        .collect();
    if lines.is_empty() {
        "无".into()
    } else {
        lines.join("\n")
    }
}

fn looks_like_meta(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("here")
        || lower.starts_with("sure")
        || line.starts_with("好的")
        || line.starts_with("可以")
        || line.starts_with("这句")
        || line.starts_with("输出")
        || line.starts_with("答案")
}

fn strip_prefix_noise(line: &str) -> &str {
    let mut text = line.trim();
    for prefix in [
        "- ",
        "* ",
        "1. ",
        "1、",
        "输出：",
        "输出:",
        "答案：",
        "答案:",
        "句子：",
        "句子:",
        "短句：",
        "短句:",
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest.trim();
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_strips_meta_and_quotes() {
        let raw = "好的，这里是一句：\n“电梯到十七层才停，镜子里的人终于把表情收回口袋。”";
        assert_eq!(
            clean(raw.into()),
            "电梯到十七层才停，镜子里的人终于把表情收回口袋。"
        );
    }

    #[test]
    fn quality_rejects_formulaic_lines() {
        let text = "真正的成熟不是学会忍耐，而是把所有委屈都交给时间。";
        assert!(quality_issue(text, &[]).is_some());
    }

    #[test]
    fn quality_rejects_recent_duplicates() {
        let text = "便利店的灯太亮，显得深夜买来的安慰也有保质期。";
        assert!(quality_issue(text, &[text.into()]).is_some());
    }

    #[test]
    fn quality_accepts_concrete_observations() {
        let text = "洗衣机停下后，房间安静得像刚刚替谁保守了一个小秘密。";
        assert!(quality_issue(text, &[]).is_none());
    }

    #[test]
    fn unknown_style_falls_back_to_commute() {
        assert_eq!(normalize_style("unknown"), "commute");
    }

    #[test]
    fn unknown_sharpness_falls_back_to_quiet() {
        assert_eq!(normalize_sharpness("loud"), "quiet");
    }
}

fn enriched_path() -> OsString {
    let mut paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let extras = ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"];
    for e in extras {
        let p = PathBuf::from(e);
        if !paths.contains(&p) {
            paths.push(p);
        }
    }
    if let Some(home) = dirs::home_dir() {
        for sub in [".local/bin", ".cargo/bin", ".bun/bin", "bin"] {
            let p = home.join(sub);
            if !paths.contains(&p) {
                paths.push(p);
            }
        }
    }
    std::env::join_paths(paths).unwrap_or_default()
}
