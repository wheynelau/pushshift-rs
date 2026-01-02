use anyhow::bail;
use rayon::str;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct Reddit {
    pub id: String,
    pub selftext: String,
    pub parent_id: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Thread {
    pub name: String,
    pub selftext: String,
    pub num_comments: u64,
}
#[derive(Deserialize, Serialize)]
pub struct Comment {
    pub name: String,
    pub body: String,
    pub parent_id: String,
    pub score: u64,
    pub ups: u64,
    pub downs: u64,
}

impl Reddit {
    #[allow(dead_code)]
    pub fn from_comment(json: &Value) -> Option<Self> {
        if let Some(body) = json.get("body") {
            // if the string is [deleted] or [removed], return None
            let body = body.as_str().unwrap();
            if body == "[deleted]" || body == "[removed]" {
                return None;
            }
            let id = json
                .get("name")
                .or_else(|| json.get("id"))
                .unwrap_or_else(|| {
                    eprintln!("Error: both 'id' and 'name' are missing. JSON: {json:?}");
                    panic!("Neither 'id' nor 'name' found in JSON");
                })
                .as_str()
                .unwrap_or_else(|| {
                    eprintln!("Error: value is not a string. JSON: {json:?}");
                    panic!("Neither 'id' nor 'name' is a string");
                });

            let parent_id = json.get("parent_id").unwrap().as_str().unwrap();
            let comment = Reddit {
                id: id.to_string(),
                selftext: body.to_string(),
                parent_id: Some(parent_id.to_string()),
            };
            Some(comment)
        } else {
            dbg!("No 'body' field found");
            None
        }
    }
    #[allow(dead_code)]
    pub fn from_thread(json: &Value) -> Option<Self> {
        if let Some(num_comments) = json.get("num_comments") {
            if num_comments == 0 {
                return None;
            }

            let id = json
                .get("name")
                .or_else(|| json.get("id"))
                .unwrap_or_else(|| {
                    eprintln!("Error: both 'id' and 'name' are missing. JSON: {json:?}");
                    panic!("Neither 'id' nor 'name' found in JSON");
                })
                .as_str()
                .unwrap_or_else(|| {
                    eprintln!("Error: value is not a string. JSON: {json:?}");
                    panic!("Neither 'id' nor 'name' is a string");
                });
            let selftext = json.get("selftext").unwrap().as_str().unwrap().to_string();

            if selftext.len() < 10 {
                return None;
            }

            let thread = Reddit {
                id: id.to_string(),
                selftext,
                parent_id: None,
            };

            Some(thread)
        } else {
            dbg!("No 'num_comments' field found");
            None
        }
    }
}

impl Comment {
    pub fn into_reddit(self, include_scores: bool) -> Result<Reddit, anyhow::Error> {
        let selftext = self.body;

        if selftext == "[deleted]" || selftext == "[removed]" {
            bail!("Comment is deleted or removed");
        }

        let selftext = if include_scores {
            let score = self.score;
            let ups = self.ups;
            let downs = self.downs;
            format!("{selftext}\n\n{{score: {score} Ups: {ups} Downs: {downs}}}")
        } else {
            selftext
        };

        Ok(Reddit {
            id: self.name,
            selftext,
            parent_id: Some(self.parent_id),
        })
    }
}

impl TryFrom<Comment> for Reddit {
    type Error = anyhow::Error;
    fn try_from(comment: Comment) -> Result<Reddit, Self::Error> {
        let selftext = comment.body;
        if selftext == "[deleted]" || selftext == "[removed]" {
            bail!("Comment is deleted or removed");
        }

        let reddit = Reddit {
            id: comment.name,
            selftext,
            parent_id: Some(comment.parent_id),
        };
        Ok(reddit)
    }
}

impl TryFrom<Thread> for Reddit {
    type Error = anyhow::Error;
    fn try_from(thread: Thread) -> Result<Reddit, Self::Error> {
        if thread.num_comments == 0 {
            bail!("Thread has no comments");
        }
        let id = thread.name;
        let selftext = thread.selftext;
        if selftext.len() < 10 {
            bail!("Thread is too short");
        }
        let reddit = Reddit {
            id,
            selftext,
            parent_id: None,
        };
        Ok(reddit)
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufRead;

    use super::*;

    #[test]
    fn test_from_comment() {
        let file = std::fs::File::open("processed/RC_2025-09.jsonl").unwrap();
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap();

            // Old method: parse to Value, then use from_comment
            let json_value: Value = serde_json::from_str(&line).unwrap();
            let reddit_old = Reddit::from_comment(&json_value);

            // New method: parse to Comment struct, then use TryFrom
            let comment: Comment = serde_json::from_str(&line).unwrap();
            let reddit_new: Option<Reddit> = comment.try_into().ok();

            // Verify both methods produce the same result
            assert_eq!(
                reddit_old, reddit_new,
                "Mismatch for comment line. Old: {reddit_old:?}, New: {reddit_new:?}"
            );
        }
    }
    #[test]
    fn test_from_thread() {
        let file = std::fs::File::open("processed/submission-R_2025-09.jsonl").unwrap();
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap();

            // Old method: parse to Value, then use from_thread
            let json_value: Value = serde_json::from_str(&line).unwrap();
            let reddit_old = Reddit::from_thread(&json_value);

            // New method: parse to Thread struct, then use TryFrom
            let thread: Thread = serde_json::from_str(&line).unwrap();
            let reddit_new: Option<Reddit> = thread.try_into().ok();

            // Verify both methods produce the same result
            assert_eq!(
                reddit_old, reddit_new,
                "Mismatch for thread line. Old: {reddit_old:?}, New: {reddit_new:?}",
            );
        }
    }
}
