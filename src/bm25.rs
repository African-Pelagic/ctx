//! Dependency-free BM25 ranking over pre-tokenised documents.
//!
//! Ranking is a pure function of the corpus and the query, so results are
//! reproducible across runs and machines.

use std::collections::{HashMap, HashSet};

const K1: f64 = 1.5;
const B: f64 = 0.75;

const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "in", "is", "it", "its",
    "of", "on", "or", "that", "the", "to", "was", "were", "which", "with",
];

/// Lowercase alphanumeric tokens (single characters and stopwords dropped) and a deliberately
/// crude plural strip ("funds" -> "fund"). Documents and queries must go
/// through the same function.
pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() > 1)
        .map(|t| t.to_lowercase())
        .filter(|t| !STOPWORDS.contains(&t.as_str()))
        .map(|t| stem(&t))
        .collect()
}

fn stem(token: &str) -> String {
    if token.len() > 3 && token.ends_with('s') && !token.ends_with("ss") {
        token[..token.len() - 1].to_string()
    } else {
        token.to_string()
    }
}

/// Returns `(document index, score)` for every document with a non-zero score,
/// best first. Ties break on document index so output order is stable.
pub fn rank(corpus: &[Vec<String>], query: &[String]) -> Vec<(usize, f64)> {
    let query: Vec<&String> = query.iter().collect::<HashSet<_>>().into_iter().collect();
    if corpus.is_empty() || query.is_empty() {
        return Vec::new();
    }

    let n = corpus.len() as f64;
    let avg_len = corpus.iter().map(Vec::len).sum::<usize>() as f64 / n;

    let mut doc_freq: HashMap<&str, usize> = HashMap::new();
    let mut term_freqs: Vec<HashMap<&str, usize>> = Vec::with_capacity(corpus.len());
    for doc in corpus {
        let mut tf: HashMap<&str, usize> = HashMap::new();
        for token in doc {
            *tf.entry(token.as_str()).or_default() += 1;
        }
        for token in tf.keys() {
            *doc_freq.entry(token).or_default() += 1;
        }
        term_freqs.push(tf);
    }

    let mut scored = Vec::new();
    for (index, tf) in term_freqs.iter().enumerate() {
        let len = corpus[index].len() as f64;
        let mut score = 0.0;
        for term in &query {
            let Some(&freq) = tf.get(term.as_str()) else {
                continue;
            };
            let df = doc_freq[term.as_str()] as f64;
            let idf = (1.0 + (n - df + 0.5) / (df + 0.5)).ln();
            let freq = freq as f64;
            score += idf * freq * (K1 + 1.0) / (freq + K1 * (1.0 - B + B * len / avg_len));
        }
        if score > 0.0 {
            scored.push((index, score));
        }
    }

    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    scored
}

#[cfg(test)]
mod tests {
    use super::{rank, tokenize};

    #[test]
    fn tokenizes_lowercase_without_stopwords_or_plurals() {
        assert_eq!(
            tokenize("The Funds of Carry, and LBO's"),
            vec!["fund", "carry", "lbo"]
        );
    }

    #[test]
    fn rarer_terms_outrank_common_ones() {
        let corpus = vec![
            tokenize("fund fund fund returns"),
            tokenize("fund hurdle rate"),
            tokenize("fund fees"),
        ];
        let hits = rank(&corpus, &tokenize("hurdle fund"));
        assert_eq!(hits[0].0, 1);
    }

    #[test]
    fn ties_break_on_document_order_and_misses_are_dropped() {
        let corpus = vec![tokenize("alpha"), tokenize("alpha"), tokenize("beta")];
        let hits = rank(&corpus, &tokenize("alpha"));
        assert_eq!(hits.iter().map(|h| h.0).collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn empty_query_or_corpus_ranks_nothing() {
        assert!(rank(&[], &tokenize("alpha")).is_empty());
        assert!(rank(&[tokenize("alpha")], &[]).is_empty());
    }
}
