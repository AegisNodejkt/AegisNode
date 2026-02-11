use regex::Regex;
use crate::core::config::PrivacyRule;
use hyper::body::HttpBody;

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub rule_name: String,
    pub match_content: String,
    pub start_index: usize,
    pub end_index: usize,
    pub action: Option<String>,
    pub replace: Option<String>,
}

pub struct PIIScanner {
    rules: Vec<PrivacyRule>,
}

impl PIIScanner {
    pub fn new(rules: Vec<PrivacyRule>) -> Self {
        Self { rules }
    }

    pub async fn scan_and_redact_request(
        &self,
        req: &mut hyper::Request<hyper::Body>,
    ) -> Result<(), String> {
        // 1. Check if we have a body
        if req.body().size_hint().exact() == Some(0) {
            return Ok(());
        }

        // 2. Read body bytes
        let body_bytes = hyper::body::to_bytes(std::mem::replace(req.body_mut(), hyper::Body::empty()))
            .await
            .map_err(|e| format!("Failed to read request body: {}", e))?;

        // 3. Check if body is valid UTF-8 (text/json)
        // If not (e.g. binary), we might skip or handle differently.
        if let Ok(body_str) = std::str::from_utf8(&body_bytes) {
            // 4. Scan and Redact
            let redacted_body_str = self.redact(body_str);
            
            // Only update if changed
            if redacted_body_str != body_str {
                println!("PIIScanner: Redacted sensitive info in request body.");
                let new_body = hyper::Body::from(redacted_body_str.clone());
                let new_len = redacted_body_str.len();
                
                *req.body_mut() = new_body;
                
                // Update Content-Length header to avoid "danger_full_buf length mismatches" panic
                req.headers_mut().insert(
                    hyper::header::CONTENT_LENGTH, 
                    hyper::header::HeaderValue::from(new_len)
                );
            } else {
                 // Restore original body if no changes
                 *req.body_mut() = hyper::Body::from(body_bytes);
            }
        } else {
             // Restore original body if not text
             *req.body_mut() = hyper::Body::from(body_bytes);
        }

        Ok(())
    }

    pub fn scan(&self, text: &str) -> Vec<ScanResult> {
        let mut results = Vec::new();

        for rule in &self.rules {
            if rule.rule_type == "pattern" {
                if let Ok(re) = Regex::new(&rule.value) {
                    for cap in re.captures_iter(text) {
                        if let Some(matched) = cap.get(0) {
                            results.push(ScanResult {
                                rule_name: rule.name.clone(),
                                match_content: matched.as_str().to_string(),
                                start_index: matched.start(),
                                end_index: matched.end(),
                                action: rule.action.clone(),
                                replace: rule.replace.clone(), // Use configured replace string
                            });
                        }
                    }
                }
            } else if rule.rule_type == "entity" {
                // Simple keyword matching for entity types if regex isn't used
                // In a real scenario, this might use a dictionary or NLP model
                // For now, we treat 'value' as a literal keyword if not a pattern
                if text.contains(&rule.value) {
                    let start = text.find(&rule.value).unwrap_or(0);
                    results.push(ScanResult {
                        rule_name: rule.name.clone(),
                        match_content: rule.value.clone(),
                        start_index: start,
                        end_index: start + rule.value.len(),
                        action: rule.action.clone(),
                        replace: rule.replace.clone(), // Use configured replace string
                    });
                }
            }
        }

        results
    }
    
    pub fn redact(&self, text: &str) -> String {
        let mut redacted_text = text.to_string();
        let findings = self.scan(text);
        
        // Sort findings by start_index in descending order to avoid offsetting indices when replacing
        let mut sorted_findings = findings;
        sorted_findings.sort_by(|a, b| b.start_index.cmp(&a.start_index));
        
        for finding in sorted_findings {
            if let Some(action) = &finding.action {
                if action == "redact" || action == "block" {
                    let replacement = finding.replace.clone().unwrap_or_else(|| "*".repeat(finding.match_content.len()));
                    redacted_text.replace_range(finding.start_index..finding.end_index, &replacement);
                }
            }
        }
        
        redacted_text
    }
}
