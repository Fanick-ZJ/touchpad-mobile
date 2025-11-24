use rand::Rng;

pub fn rand_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&rand::distr::Alphabetic)
        .take(len)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rand_string() {
        let s = rand_string(10);
        println!("{}", s);
        assert_eq!(s.len(), 10);
        assert!(s.chars().all(|c| c.is_alphabetic()));
    }
}
