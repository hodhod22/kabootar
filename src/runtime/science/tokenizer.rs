//! Tokenizer subset — word vocab + BPE-lite (SC2k).

use super::helpers::{int_out, num};
use crate::value::{Environment, Value};
use std::collections::{HashMap, HashSet};

fn split_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| w.to_lowercase())
        .filter(|w| !w.is_empty())
        .collect()
}

fn vocab_out(token_to_id: &HashMap<String, i64>) -> Value {
    let mut id_to_token: Vec<(i64, String)> = token_to_id
        .iter()
        .map(|(t, id)| (*id, t.clone()))
        .collect();
    id_to_token.sort_by_key(|(id, _)| *id);
    let mut out = HashMap::new();
    out.insert(
        "token_to_id".into(),
        Value::Object(
            token_to_id
                .iter()
                .map(|(k, v)| (k.clone(), int_out(*v)))
                .collect(),
        ),
    );
    out.insert(
        "id_to_token".into(),
        Value::Array(
            id_to_token
                .into_iter()
                .map(|(_, t)| Value::String(t))
                .collect(),
        ),
    );
    out.insert("size".into(), int_out(token_to_id.len() as i64));
    Value::Object(out)
}

/// tok_build_vocab(texts[], maxVocab?) — whitespace word vocab.
fn tok_build_vocab(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let texts = match args.first() {
        Some(Value::Array(items)) => items,
        _ => return Err("tok_build_vocab(texts, maxVocab?)".into()),
    };
    let max_vocab = args
        .get(1)
        .and_then(|v| num(v).ok())
        .unwrap_or(10000.0) as usize;
    let mut counts: HashMap<String, i64> = HashMap::new();
    for t in texts {
        let s = match t {
            Value::String(x) => x.clone(),
            _ => continue,
        };
        for w in split_words(&s) {
            counts.insert(w.clone(), counts.get(&w).copied().unwrap_or(0) + 1);
        }
    }
    let mut ranked: Vec<(String, i64)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut token_to_id = HashMap::new();
    token_to_id.insert("<pad>".into(), 0);
    token_to_id.insert("<unk>".into(), 1);
    for (w, _) in ranked.into_iter().take(max_vocab.saturating_sub(2)) {
        if !token_to_id.contains_key(&w) {
            let id = token_to_id.len() as i64;
            token_to_id.insert(w, id);
        }
    }
    Ok(vocab_out(&token_to_id))
}

fn vocab_lookup(vocab: &Value) -> Result<HashMap<String, i64>, String> {
    match vocab {
        Value::Object(m) => {
            if let Some(Value::Object(map)) = m.get("token_to_id") {
                let mut out = HashMap::new();
                for (k, v) in map {
                    out.insert(k.clone(), num(v)? as i64);
                }
                return Ok(out);
            }
            let mut out = HashMap::new();
            for (k, v) in m {
                out.insert(k.clone(), num(v)? as i64);
            }
            Ok(out)
        }
        _ => Err("tok: expected vocab object".into()),
    }
}

fn id_lookup(vocab: &Value) -> Result<Vec<String>, String> {
    match vocab {
        Value::Object(m) => {
            if let Some(Value::Array(items)) = m.get("id_to_token") {
                return Ok(
                    items
                        .iter()
                        .map(|x| match x {
                            Value::String(s) => s.clone(),
                            _ => "<unk>".into(),
                        })
                        .collect(),
                );
            }
            let map = vocab_lookup(vocab)?;
            let max_id = map.values().copied().max().unwrap_or(0);
            let mut id_to_token = vec!["<unk>".to_string(); max_id as usize + 1];
            for (t, id) in map {
                if id >= 0 && (id as usize) < id_to_token.len() {
                    id_to_token[id as usize] = t;
                }
            }
            Ok(id_to_token)
        }
        _ => Err("tok: expected vocab".into()),
    }
}

/// tok_encode(vocab, text) → ids[]
fn tok_encode(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let vocab = args.first().ok_or("tok_encode(vocab, text)")?;
    let text = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("tok_encode: text string".into()),
    };
    let map = vocab_lookup(vocab)?;
    let unk = map.get("<unk>").copied().unwrap_or(1);
    let ids: Vec<Value> = split_words(text)
        .into_iter()
        .map(|w| {
            int_out(map.get(&w).copied().unwrap_or(unk))
        })
        .collect();
    Ok(Value::Array(ids))
}

/// tok_decode(vocab, ids) → string
fn tok_decode(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let vocab = args.first().ok_or("tok_decode(vocab, ids)")?;
    let ids = match args.get(1) {
        Some(Value::Array(items)) => items,
        _ => return Err("tok_decode: ids array".into()),
    };
    let id_to_token = id_lookup(vocab)?;
    let mut parts = Vec::new();
    for idv in ids {
        let id = num(idv)? as i64;
        let t = id_to_token
            .get(id as usize)
            .cloned()
            .unwrap_or_else(|| "<unk>".to_string());
        if t != "<pad>" {
            parts.push(t);
        }
    }
    Ok(Value::String(parts.join(" ")))
}

fn word_to_chars(word: &str) -> Vec<String> {
    let mut tokens: Vec<String> = word.chars().map(|c| c.to_string()).collect();
    if tokens.len() > 1 {
        let last = tokens.pop().unwrap();
        tokens.push(format!("{}</w>", last));
    } else if let Some(t) = tokens.first_mut() {
        *t = format!("{}/w>", t);
    }
    tokens
}

/// tok_bpe_train(texts[], numMerges?, maxVocab?) → {vocab, merges}
fn tok_bpe_train(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let texts = match args.first() {
        Some(Value::Array(items)) => items,
        _ => return Err("tok_bpe_train(texts, numMerges?, maxVocab?)".into()),
    };
    let num_merges = args
        .get(1)
        .and_then(|v| num(v).ok())
        .unwrap_or(50.0) as usize;
    let max_vocab = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(5000.0) as usize;

    let mut corpus: Vec<Vec<String>> = Vec::new();
    for t in texts {
        let s = match t {
            Value::String(x) => x.clone(),
            _ => continue,
        };
        for w in split_words(&s) {
            corpus.push(word_to_chars(&w));
        }
    }
    if corpus.is_empty() {
        return Err("tok_bpe_train: empty corpus".into());
    }

    let mut merges: Vec<(String, String)> = Vec::new();
    for _ in 0..num_merges {
        let mut pair_counts: HashMap<(String, String), i64> = HashMap::new();
        for word in &corpus {
            for i in 0..word.len().saturating_sub(1) {
                let pair = (word[i].clone(), word[i + 1].clone());
                pair_counts.insert(pair.clone(), pair_counts.get(&pair).copied().unwrap_or(0) + 1);
            }
        }
        if pair_counts.is_empty() {
            break;
        }
        let best = pair_counts
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.0.cmp(&a.0.0)))
            .map(|(p, _)| p);
        let Some((a, b)) = best else { break };
        let merged = format!("{}{}", a, b);
        merges.push((a.clone(), b.clone()));
        for word in &mut corpus {
            let mut i = 0;
            while i < word.len().saturating_sub(1) {
                if word[i] == a && word[i + 1] == b {
                    word[i] = merged.clone();
                    word.remove(i + 1);
                } else {
                    i += 1;
                }
            }
        }
    }

    let mut token_set: HashSet<String> = HashSet::new();
    token_set.insert("<pad>".into());
    token_set.insert("<unk>".into());
    for word in &corpus {
        for t in word {
            token_set.insert(t.clone());
        }
    }
    let mut tokens: Vec<String> = token_set.into_iter().collect();
    tokens.sort();
    if tokens.len() > max_vocab {
        tokens.truncate(max_vocab);
    }
    let mut token_to_id = HashMap::new();
    for (i, t) in tokens.iter().enumerate() {
        token_to_id.insert(t.clone(), i as i64);
    }

    let mut merge_arr = Vec::new();
    for (a, b) in merges {
        merge_arr.push(Value::Array(vec![
            Value::String(a),
            Value::String(b),
        ]));
    }

    let mut out = HashMap::new();
    out.insert("vocab".into(), vocab_out(&token_to_id));
    out.insert("merges".into(), Value::Array(merge_arr));
    Ok(Value::Object(out))
}

fn bpe_apply(word: &str, merges: &[(String, String)]) -> Vec<String> {
    let mut tokens = word_to_chars(word);
    for (a, b) in merges {
        let merged = format!("{}{}", a, b);
        let mut i = 0;
        while i < tokens.len().saturating_sub(1) {
            if tokens[i] == *a && tokens[i + 1] == *b {
                tokens[i] = merged.clone();
                tokens.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }
    tokens
}

fn parse_merges(v: &Value) -> Result<Vec<(String, String)>, String> {
    match v {
        Value::Array(items) => items
            .iter()
            .map(|pair| {
                let Value::Array(cells) = pair else {
                    return Err("tok_bpe_encode: merge pair".into());
                };
                if cells.len() != 2 {
                    return Err("tok_bpe_encode: merge pair len".into());
                }
                let a = match &cells[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err("tok_bpe_encode: merge a".into()),
                };
                let b = match &cells[1] {
                    Value::String(s) => s.clone(),
                    _ => return Err("tok_bpe_encode: merge b".into()),
                };
                Ok((a, b))
            })
            .collect(),
        _ => Err("tok_bpe_encode: merges array".into()),
    }
}

/// tok_bpe_encode(model, text) — model = {vocab, merges}
fn tok_bpe_encode(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let model = match args.first() {
        Some(Value::Object(m)) => m,
        _ => return Err("tok_bpe_encode(model, text)".into()),
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("tok_bpe_encode: text".into()),
    };
    let vocab = model.get("vocab").ok_or("tok_bpe_encode: vocab")?;
    let merges_v = model.get("merges").ok_or("tok_bpe_encode: merges")?;
    let map = vocab_lookup(vocab)?;
    let merges = parse_merges(merges_v)?;
    let unk = map.get("<unk>").copied().unwrap_or(1);

    let mut ids = Vec::new();
    for w in split_words(text) {
        let tokens = bpe_apply(&w, &merges);
        for t in tokens {
            ids.push(int_out(map.get(&t).copied().unwrap_or(unk)));
        }
    }
    Ok(Value::Array(ids))
}

/// tok_bpe_decode(model, ids)
fn tok_bpe_decode(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let model = match args.first() {
        Some(Value::Object(m)) => m,
        _ => return Err("tok_bpe_decode(model, ids)".into()),
    };
    let ids = match args.get(1) {
        Some(Value::Array(items)) => items,
        _ => return Err("tok_bpe_decode: ids".into()),
    };
    let vocab = model.get("vocab").ok_or("tok_bpe_decode: vocab")?;
    let id_to_token = id_lookup(vocab)?;
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    for idv in ids {
        let id = num(idv)? as i64;
        let t = id_to_token
            .get(id as usize)
            .cloned()
            .unwrap_or_else(|| "<unk>".to_string());
        if t == "<pad>" {
            continue;
        }
        if t.ends_with("</w>") {
            current.push_str(&t.trim_end_matches("</w>"));
            words.push(current.clone());
            current.clear();
        } else {
            current.push_str(&t);
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    Ok(Value::String(words.join(" ")))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_tok_build_vocab", "tok_build_vocab"], tok_build_vocab);
    bind(&["science_tok_encode", "tok_encode"], tok_encode);
    bind(&["science_tok_decode", "tok_decode"], tok_decode);
    bind(&["science_tok_bpe_train", "tok_bpe_train"], tok_bpe_train);
    bind(&["science_tok_bpe_encode", "tok_bpe_encode"], tok_bpe_encode);
    bind(&["science_tok_bpe_decode", "tok_bpe_decode"], tok_bpe_decode);
}
