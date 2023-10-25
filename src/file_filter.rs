pub mod file_filter {
    use std::path::Path;
    use std::fs::File;
    use regex::RegexSet;
    use std::io::{BufRead, Error, BufReader};

    pub struct FileFilter {
        patterns: RegexSet,
    }

    impl FileFilter {

        pub fn from_gitignore(gitignore: &Path) -> Result<FileFilter, Error> {
            let file = File::open(gitignore).expect("Unable to open file!");
            let reader = BufReader::new(file);
            let mut regex_patterns: Vec<String> = Vec::new();

            for line in reader.lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(_) => continue,
                };
                if line.starts_with('#') {
                    continue;
                }
                if line.len() == 0 {
                    continue;
                }
                regex_patterns.push(line);
            }
            return Ok(FileFilter {
                patterns: RegexSet::new(regex_patterns).expect("Unable to create regex"),
            });
        }

        pub fn matches(&self, path: &Path) -> Result<bool, ()> {
            let name = match path.file_name() {
                Some(name) => name,
                None => return Err(())
            };
            let name = match name.to_str(){
                Some(name) => name,
                None => return Err(())
            };
            let matches = self.patterns.matches(name);
            return Ok(matches.matched_any());
        }
    }
}

mod tests {
    use std::path::Path;

    use super::file_filter::FileFilter;
    #[test]
    fn test_load() {

        let gitignore = Path::new("dedup_ignore.txt");
        assert!(gitignore.exists());
        let dut = FileFilter::from_gitignore(gitignore).unwrap();
        assert!(dut.matches(Path::new("desktop.ini")).is_ok());
        assert!(dut.matches(Path::new("Thumbs.db")).is_ok());
        assert!(dut.matches(Path::new(".DS_Store")).is_ok());
    }
}
