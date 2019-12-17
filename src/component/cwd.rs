use crate::component::Component;
use git2::Repository;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum CwdStyle {
    Default,
    Long,
    Short,
}

pub fn display(
    style: &CwdStyle,
    current_dir: &PathBuf,
    repository: Option<&Repository>,
) -> Component {
    let output = match style {
        CwdStyle::Default => {
            let home_dir = dirs::home_dir().unwrap_or(PathBuf::new());
            replace_home_dir(current_dir, home_dir)
        }
        CwdStyle::Short => {
            let home_dir = dirs::home_dir().unwrap_or(PathBuf::new());
            match repository {
                Some(repository) => {
                    short(&current_dir, &home_dir, &repository.path().to_path_buf())
                }
                // TODO: We want to contract up to the current dir if we don't have a git root.
                None => replace_home_dir(current_dir, home_dir),
            }
        }
        CwdStyle::Long => format!("{}", current_dir.display()),
    };

    Component::Cwd(format!("{}", output))
}

/// Replace the home directory portion of the path with "~/"
fn replace_home_dir(current_dir: &PathBuf, home_dir: PathBuf) -> String {
    if current_dir == &home_dir {
        return "~".to_string();
    }

    match current_dir.strip_prefix(home_dir) {
        Ok(current_dir) => format!("~/{}", current_dir.display()),
        // Unable to strip the prefix, fall back to full path
        Err(_) => format!("{}", current_dir.display()),
    }
}

fn short(current_dir: &PathBuf, home_dir: &PathBuf, git_root: &Path) -> String {
    match current_dir.strip_prefix(&home_dir) {
        Ok(current_dir) => {
            // Remove repo/.git
            let git_root = git_root.parent().unwrap().parent().unwrap();
            // Remove the home_dir from the git_root.
            let git_root = git_root
                .strip_prefix(&home_dir)
                .expect("unable to remove home dir");

            let short_repo = git_root.iter().fold(PathBuf::new(), |acc, part| {
                acc.join(format!("{}", part.to_string_lossy().chars().nth(0).unwrap()).as_str())
            });

            let rest = current_dir
                .strip_prefix(&git_root)
                .expect("unable to remove non-home-dir git_root from dir");

            let mut output = PathBuf::new();
            output.push(&home_dir);
            output.push(short_repo);
            output.push(rest);

            replace_home_dir(&output, home_dir.to_path_buf())
        }
        // Unable to strip the prefix, fall back to full path
        Err(_) => format!("{}", current_dir.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_home_dir() {
        let current_dir = PathBuf::from("/home/foo/bar/baz");
        let home_dir = PathBuf::from("/home/foo");

        assert_eq!(
            replace_home_dir(&current_dir, home_dir),
            "~/bar/baz".to_string()
        );
    }

    #[test]
    fn test_replace_home_dir_in_home_dir() {
        let current_dir = PathBuf::from("/home/foo");
        let home_dir = PathBuf::from("/home/foo");

        assert_eq!(replace_home_dir(&current_dir, home_dir), "~".to_string());
    }

    #[test]
    fn short_test() {
        let current_dir = PathBuf::from("/home/foo/axx/bxx/repo/cxx/dxx");
        let home_dir = PathBuf::from("/home/foo");
        let git_root = Path::new("/home/foo/axx/bxx/repo/.git");

        assert_eq!(
            short(&current_dir, &home_dir, &git_root),
            "~/a/b/repo/cxx/dxx".to_string()
        );

        let current_dir = PathBuf::from("/home/foo/axx/bxx/repo");
        assert_eq!(
            short(&current_dir, &home_dir, &git_root),
            "~/a/b/repo".to_string()
        );
    }

    #[test]
    fn short_test_single_dir_repo() {
        let current_dir = PathBuf::from("/home/foo/axx");
        let home_dir = PathBuf::from("/home/foo");
        let git_root = Path::new("/home/foo/axx/.git");

        assert_eq!(
            short(&current_dir, &home_dir, &git_root),
            "~/axx".to_string()
        );
    }
}
