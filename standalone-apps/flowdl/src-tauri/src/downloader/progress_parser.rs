use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedProgress {
    pub percent: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bytes_per_sec: Option<f64>,
    pub eta_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputLineEvent {
    Progress(ParsedProgress),
    PostprocessStage(String),
    DestinationFile(String),
    FinalFilepath(String),
    ErrorLine(String),
    InfoLine,
}

pub fn parse_ytdlp_line(line: &str) -> OutputLineEvent {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return OutputLineEvent::InfoLine;
    }

    // 1. Custom progress template:
    // FLOWDL_PROGRESS|<percent>|<downloaded>|<total>|<total_est>|<speed>|<eta>
    if let Some(rest) = trimmed.strip_prefix("FLOWDL_PROGRESS|") {
        let parts: Vec<&str> = rest.split('|').collect();
        if parts.len() >= 6 {
            let pct_raw = parts[0].trim().trim_end_matches('%').trim();
            let downloaded = parts[1].trim().parse::<f64>().ok().map(|v| v as u64).unwrap_or(0);
            let total_exact = parts[2].trim().parse::<f64>().ok().map(|v| v as u64);
            let total_est = parts[3].trim().parse::<f64>().ok().map(|v| v as u64);
            let total = total_exact.or(total_est).filter(|&v| v > 0);

            let percent = pct_raw
                .parse::<f64>()
                .ok()
                .or_else(|| {
                    total.map(|t| ((downloaded as f64 / t as f64) * 100.0).clamp(0.0, 99.9))
                })
                .unwrap_or(0.0)
                .clamp(0.0, 99.9);

            let speed = parts[4]
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|&s| s > 0.0 && s.is_finite());

            let eta = parts[5]
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|&e| e >= 0.0 && e.is_finite())
                .map(|e| e.round() as u64);

            return OutputLineEvent::Progress(ParsedProgress {
                percent,
                downloaded_bytes: downloaded,
                total_bytes: total,
                speed_bytes_per_sec: speed,
                eta_seconds: eta,
            });
        }
    }

    // 2. Final filepath marker:
    if let Some(filepath) = trimmed.strip_prefix("FLOWDL_FILEPATH|") {
        let path_clean = filepath.trim().to_string();
        if !path_clean.is_empty() && path_clean != "NA" {
            return OutputLineEvent::FinalFilepath(path_clean);
        }
    }

    // 3. Detect destination stream type or postprocessing stage
    if let Some(dest) = trimmed.strip_prefix("[download] Destination:") {
        let d = dest.trim().to_string();
        let lower = d.to_lowercase();
        if lower.ends_with(".m4a")
            || lower.ends_with(".webm") && lower.contains(".f25")
            || lower.ends_with(".mp3")
            || lower.ends_with(".opus")
        {
            return OutputLineEvent::PostprocessStage("downloading_audio".to_string());
        }
        return OutputLineEvent::DestinationFile(d);
    }

    if trimmed.starts_with("[Merger]") || trimmed.contains("Merging formats into") {
        if let Some(idx) = trimmed.find("Merging formats into \"") {
            let after = &trimmed[idx + 22..];
            if let Some(end_q) = after.rfind('"') {
                return OutputLineEvent::FinalFilepath(after[..end_q].to_string());
            }
        }
        return OutputLineEvent::PostprocessStage("merging_streams".to_string());
    }

    if trimmed.starts_with("[ExtractAudio]") || trimmed.starts_with("[AudioConvert]") {
        if let Some(idx) = trimmed.find("Destination: ") {
            let path = trimmed[idx + 13..].trim().to_string();
            if !path.is_empty() {
                return OutputLineEvent::FinalFilepath(path);
            }
        }
        return OutputLineEvent::PostprocessStage("converting_audio".to_string());
    }

    if trimmed.starts_with("[Metadata]")
        || trimmed.starts_with("[EmbedThumbnail]")
        || trimmed.starts_with("[ThumbnailsConvertor]")
    {
        return OutputLineEvent::PostprocessStage("embedding_metadata".to_string());
    }

    if trimmed.starts_with("[Fixup") || trimmed.starts_with("[MoveFiles]") {
        return OutputLineEvent::PostprocessStage("finalizing".to_string());
    }

    if trimmed.starts_with("ERROR:") {
        return OutputLineEvent::ErrorLine(trimmed.to_string());
    }

    OutputLineEvent::InfoLine
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_flowdl_progress_line() {
        let line = "FLOWDL_PROGRESS| 64.0%|163577856|255852544|NA|8808038.4|11";
        let evt = parse_ytdlp_line(line);
        match evt {
            OutputLineEvent::Progress(p) => {
                assert!((p.percent - 64.0).abs() < 0.01);
                assert_eq!(p.downloaded_bytes, 163577856);
                assert_eq!(p.total_bytes, Some(255852544));
                assert_eq!(p.eta_seconds, Some(11));
            }
            _ => panic!("Expected Progress event"),
        }
    }

    #[test]
    fn test_parse_postprocess_stages() {
        assert_eq!(
            parse_ytdlp_line("[Merger] Merging formats into \"C:\\Downloads\\Video.mp4\""),
            OutputLineEvent::FinalFilepath("C:\\Downloads\\Video.mp4".to_string())
        );
        assert_eq!(
            parse_ytdlp_line("[EmbedThumbnail] Adding thumbnail to \"C:\\Downloads\\Video.mp4\""),
            OutputLineEvent::PostprocessStage("embedding_metadata".to_string())
        );
    }
}
