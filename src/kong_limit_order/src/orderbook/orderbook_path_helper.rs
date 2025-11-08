use std::collections::{HashMap, VecDeque};

use crate::orderbook::{
    book_name::BookName,
    orderbook_path::{Path, BORDER_PATHS, TOKEN_PAIRS, TOKEN_PATHS},
};

fn add_token_pair_impl(v: &mut Vec<String>, token: String) -> Result<(), String> {
    if v.contains(&token) {
        return Err(format!("Pair exists"));
    }

    v.push(token);
    v.sort();

    Ok(())
}

fn add_token_pair_in_token_pairs(token_0: String, token_1: String, token_pairs: &mut HashMap<String, Vec<String>>) -> Result<(), String> {
    let v0 = token_pairs.entry(token_0.clone()).or_insert_with(Vec::new);
    add_token_pair_impl(v0, token_1.clone())?;

    let v1 = token_pairs.entry(token_1).or_insert_with(Vec::new);
    add_token_pair_impl(v1, token_0)?;

    Ok(())
}

pub fn add_token_pair(token_0: String, token_1: String) -> Result<(), String> {
    TOKEN_PAIRS.with_borrow_mut(|token_pairs| add_token_pair_in_token_pairs(token_0, token_1, token_pairs))
}

fn remove_token_pair_from_token_pairs(token_0: String, token_1: String, token_pairs: &mut HashMap<String, Vec<String>>) {
    let v = token_pairs.entry(token_0.clone()).or_insert_with(Vec::new);
    match v.iter().position(|t| *t == token_1) {
        Some(pos) => v.remove(pos),
        None => todo!(),
    };

    let v = token_pairs.entry(token_1.clone()).or_insert_with(Vec::new);
    match v.iter().position(|t| *t == token_0) {
        Some(t) => v.remove(t),
        None => todo!(),
    };
}

pub fn remove_token_pair(token_0: String, token_1: String) {
    TOKEN_PAIRS.with_borrow_mut(|token_pairs| remove_token_pair_from_token_pairs(token_0, token_1, token_pairs));
}

fn add_new_path_impl(token_paths: &mut HashMap<BookName, Vec<Path>>, border_paths: &mut HashMap<BookName, Vec<Path>>, path: Path) -> bool {
    let book_name = path.get_book_name();
    let tokens_path = token_paths.entry(book_name).or_insert_with(Vec::new);
    if tokens_path.contains(&path) {
        // ic_cdk::println!("Path already exists: {}", path.to_symbol_sequence().join(", "));
        false
    } else {
        tokens_path.push(path.clone());
        tokens_path.sort_by_key(|v| v.0.len());

        border_paths.entry(path.0.first().unwrap().clone()).or_insert_with(Vec::new).push(path.clone());
        if path.0.len() > 1 {
            border_paths.entry(path.0.last().unwrap().clone()).or_insert_with(Vec::new).push(path);
        }
        
        true
    }
}

fn add_new_path(token_paths: &mut HashMap<BookName, Vec<Path>>, border_paths: &mut HashMap<BookName, Vec<Path>>, path: Path) -> bool {
    path.check_path_valid();
    add_new_path_impl(token_paths, border_paths, path.clone())
    // Add reverse path
    // add_new_path_impl(token_paths, path.reversed());
}

fn dfs_impl(
    added_book_name: BookName,
    token_paths: &mut HashMap<BookName, Vec<Path>>,
    token_pairs: &HashMap<String, Vec<String>>,
    border_paths: &mut HashMap<BookName, Vec<Path>>,
    max_hops: usize,
) {
    // let mut visited_tokens: HashSet<String> = HashSet::new();
    let mut paths_to_continue: VecDeque<Path> = VecDeque::new();

    paths_to_continue.push_back(Path::from_book_name(added_book_name.reversed()));
    paths_to_continue.push_back(Path::from_book_name(added_book_name));

    while let Some(p) = paths_to_continue.pop_front() {
        let last_token = p.receive_token().to_string();
        // visited_tokens.insert(last_token.clone());
        if !add_new_path(token_paths, border_paths, p.clone()) {
            continue;
        }
        if p.0.len() == max_hops {
            continue;
        }

        let next_tokens = match token_pairs.get(&last_token) {
            Some(tokens) => tokens,
            None => {
                ic_cdk::eprintln!("Unexpectedly empty token pairs for token {}", last_token);
                continue;
            }
        };

        for next_token in next_tokens {
            // if visited_tokens.contains(next_token) {
            //     continue;
            // }
            // visited_tokens.insert(next_token.clone());
            let bookname = BookName::new(next_token, &last_token);

            let new_path = p.with_added_step(bookname);
            if !new_path.has_loops() {
                paths_to_continue.push_back(new_path.clone());
                paths_to_continue.push_back(new_path.reversed().clone());
            }
        }
    }
}

pub fn add_to_synth_path(token_0: &str, token_1: &str, max_hops: usize) {
    let added_book_name = BookName::new(token_0, token_1);

    TOKEN_PATHS.with_borrow_mut(|token_paths| {
        TOKEN_PAIRS.with_borrow(|token_pairs| 
            {
                BORDER_PATHS.with_borrow_mut(|border_paths| {
                    dfs_impl(added_book_name, token_paths, token_pairs, border_paths, max_hops)
                })
            })
    });
}

fn remove_from_token_paths(token_0: &str, token_1: &str, token_paths: &mut HashMap<BookName, Vec<Path>>) {
    let p1 = BookName::new(token_0, token_1);
    let p2 = BookName::new(token_1, token_0);
    for token_path in token_paths.values_mut() {
        // will contain max one of them
        match token_path
            .iter()
            .position(|v| v.contains_book_name(&p1) || v.contains_book_name(&p2))
        {
            Some(pos) => {
                let _ = token_path.remove(pos);
            }
            None => {}
        }
    }

    token_paths.retain(|_, v| !v.is_empty());
}

pub fn remove_from_synth_path(token_0: &str, token_1: &str) {
    TOKEN_PATHS.with_borrow_mut(|token_paths| { remove_from_token_paths(token_0, token_1, token_paths);});
}

#[cfg(test)]
mod tests {
    const TEST_MAX_HOPS: usize = 3;

    use super::*;

    struct TestCase {
        token_pairs: HashMap<String, Vec<String>>,
        token_paths: HashMap<BookName, Vec<Path>>,
        border_paths: HashMap<BookName, Vec<Path>>,
    }

    impl TestCase {
        fn new() -> Self {
            Self {
                token_pairs: HashMap::new(),
                token_paths: HashMap::new(),
                border_paths: HashMap::new(),
            }
        }

        fn add_new_book(&mut self, token_0: &str, token_1: &str) {
            let book_name = BookName::new(token_0, token_1);
            add_token_pair_in_token_pairs(
                book_name.receive_token().to_string(),
                book_name.send_token().to_string(),
                &mut self.token_pairs,
            )
            .unwrap();
            dfs_impl(book_name, &mut self.token_paths, &self.token_pairs, &mut self.border_paths, TEST_MAX_HOPS)
        }

        fn remove_book(&mut self, token_0: &str, token_1: &str) {
            let book_name = BookName::new(token_0, token_1);
            remove_token_pair_from_token_pairs(
                book_name.receive_token().to_string(),
                book_name.send_token().to_string(),
                &mut self.token_pairs,
            );
            remove_from_token_paths(token_0, token_1, &mut self.token_paths);
        }

        fn path_exists_one_way(&self, path: Vec<&str>) -> bool {
            let book_name = BookName::new(path.last().unwrap(), path.first().unwrap());
            let paths = match self.token_paths.get(&book_name) {
                Some(b) => b,
                None => return false,
            };

            for p in paths {
                if p.to_symbol_sequence() == path {
                    return true;
                }
            }

            return false;
        }

        fn check_path_exists_helper(&self, path: Vec<&str>, expected: bool) {
            fn expect_failed(path: Vec<&str>, expected: bool) {
                let book_name = BookName::new(path.last().unwrap(), path.first().unwrap());
                if expected {
                    assert!(false, "path {:?} does not exist in orderbook {}", path, book_name);
                } else {
                    assert!(false, "path {:?} exists in orderbook {}", path, book_name);
                }
            }

            if self.path_exists_one_way(path.clone()) != expected {
                expect_failed(path.clone(), expected);
            }

            let mut path = path.clone();
            path.reverse();
            if self.path_exists_one_way(path.clone()) != expected {
                expect_failed(path.clone(), expected);
            }
        }

        fn check_path_exists(&self, path: Vec<&str>) {
            self.check_path_exists_helper(path, true);
        }

        fn check_path_does_not_exist(&self, path: Vec<&str>) {
            self.check_path_exists_helper(path, false);
        }
    }

    #[test]
    fn test_one_edge() {
        let mut t = TestCase::new();
        t.add_new_book("a", "b");
        t.check_path_exists(vec!["a", "b"]);
    }

    #[test]
    fn test_two_edges() {
        let mut t = TestCase::new();
        t.add_new_book("a", "b");
        t.add_new_book("a", "c");
        t.check_path_exists(vec!["a", "b"]);
        t.check_path_exists(vec!["a", "c"]);
        t.check_path_exists(vec!["c", "a", "b"]);
    }

    #[test]
    fn test_loop() {
        let mut t = TestCase::new();
        t.add_new_book("a", "b");
        t.add_new_book("a", "c");
        t.add_new_book("b", "c");
        t.check_path_exists(vec!["a", "b"]);
        t.check_path_exists(vec!["a", "c"]);
        t.check_path_exists(vec!["b", "c"]);
        t.check_path_exists(vec!["a", "b", "c"]);
        t.check_path_exists(vec!["a", "c", "b"]);
        t.check_path_exists(vec!["b", "a", "c"]);
    }

    #[test]
    fn test_merge_two_components() {
        let mut t = TestCase::new();
        t.add_new_book("a", "b");
        t.add_new_book("a", "c");
        t.add_new_book("b", "c");

        t.add_new_book("a0", "b0");
        t.add_new_book("a0", "c0");
        t.add_new_book("b0", "c0");

        t.add_new_book("a", "a0");

        t.check_path_exists(vec!["a", "a0"]);
        t.check_path_exists(vec!["a", "a0", "b0"]);
        t.check_path_exists(vec!["a", "a0", "c0"]);
        t.check_path_exists(vec!["a0", "a", "b"]);
        t.check_path_exists(vec!["a0", "a", "c"]);

        t.check_path_exists(vec!["a", "a0", "b0", "c0"]);
        t.check_path_exists(vec!["a", "a0", "c0", "b0"]);
        t.check_path_exists(vec!["a0", "a", "b", "c"]);
        t.check_path_exists(vec!["a0", "a", "c", "b"]);

        t.check_path_exists(vec!["c0", "a0", "a", "b"]);
        t.check_path_exists(vec!["c0", "a0", "a", "c"]);
        t.check_path_exists(vec!["b0", "a0", "a", "b"]);
        t.check_path_exists(vec!["b0", "a0", "a", "c"]);
    }
    #[test]
    fn test_remove_one_edge() {
        let mut t = TestCase::new();
        t.add_new_book("a", "b");
        assert!(t.token_paths.contains_key(&BookName::new("a", "b")));
        assert!(t.token_paths.contains_key(&BookName::new("b", "a")));

        t.remove_book("a", "b");
        t.check_path_does_not_exist(vec!["a", "b"]);

        assert!(!t.token_paths.contains_key(&BookName::new("a", "b")));
        assert!(!t.token_paths.contains_key(&BookName::new("b", "a")));
    }

    #[test]
    fn test_remove_two_edges() {
        let mut t = TestCase::new();
        t.add_new_book("a", "b");
        t.add_new_book("a", "c");
        t.remove_book("a", "c");
        t.check_path_does_not_exist(vec!["a", "c"]);
        t.check_path_does_not_exist(vec!["c", "a", "b"]);
    }

    #[test]
    fn test_remove_loop() {
        let mut t = TestCase::new();
        t.add_new_book("a", "b");
        t.add_new_book("a", "c");
        t.add_new_book("b", "c");
        t.remove_book("a", "c");
        t.check_path_exists(vec!["a", "b"]);
        t.check_path_does_not_exist(vec!["a", "c"]);
        t.check_path_exists(vec!["b", "c"]);
        t.check_path_exists(vec!["a", "b", "c"]);
        t.check_path_does_not_exist(vec!["a", "c", "b"]);
        t.check_path_does_not_exist(vec!["b", "a", "c"]);
    }
}
