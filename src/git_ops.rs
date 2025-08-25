use git2::{IndexAddOption, Repository};

pub fn git_sync(user_dotfiles: &str) {
    let repo = match Repository::open(user_dotfiles) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open: {}", e),
    };

    let mut index = repo.index().unwrap();

    let _ = index.add_all(["."].iter(), IndexAddOption::DEFAULT, None);
    let _ = index.write();

    println!("DEBUG: Index has {} entries", index.len());

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    let parent_commit = match repo.head() {
        Ok(head) => {
            let target = head.target().unwrap();
            Some(repo.find_commit(target).unwrap())
        }
        Err(_) => None, // No previous commits (initial commit)
    };

    let message: &str = "Oxidots: update";

    let commit = repo.commit(
        Some("HEAD"),
        &repo.signature().unwrap(),
        &repo.signature().unwrap(),
        message,
        &tree,
        &[&parent_commit.unwrap()], // @TODO: Will panic if no parent commit
    );

    println!("DEBUG COMMIT {:?}", commit);
}

