extern crate yaml_rust;
extern crate notify;
extern crate chan;
extern crate chan_signal;

use chan_signal::Signal;
use threadpool::ThreadPool;

use std::io;
use std::fs;
use std::path::PathBuf;
use yaml_rust::YamlLoader;

use notify::{Watcher, RecursiveMode, watcher};

use std::sync::mpsc::channel;
use std::time::Duration;

#[derive(Debug)]
struct Directory {
    source: String,
    target: String,
}

impl Directory {
    pub fn watch(&self) {
        println!("{} syncing...", self.source);

        self.sync_files().unwrap();

        println!("{} synced!", self.source);

        let (tx, rx) = channel();

        let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

        watcher.watch(&self.source, RecursiveMode::Recursive).unwrap();

        println!("Watching {}", self.source);

        loop {
            match rx.recv() {
                Ok(event) => {
                    match event {
                        notify::DebouncedEvent::Create(path) => {
                            println!("Create {:?}", path);
                            self.path_create(path);
                        },
                        notify::DebouncedEvent::Write(path) => {
                            println!("Write {:?}", path);
                            self.path_write(path);
                        },
                        notify::DebouncedEvent::Chmod(path) => {
                            println!("Chmod {:?}", path);
                            self.path_chmod(path);
                        },
                        notify::DebouncedEvent::Remove(path) => {
                            println!("Remove {:?}", path);
                            self.path_remove(path);
                        },
                        notify::DebouncedEvent::Rename(from, to) => {
                            println!("Rename {:?} to {:?}", from, to);
                            self.path_rename(from, to);
                        },
                        notify::DebouncedEvent::Error(error, _path) => {
                            println!("Error! {:?}", error);
                        },
                        _ => (),
                    }
                },
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    }

    fn path_create(&self, path: PathBuf) {
        let target_path = self.build_target_path(&path);

        println!("Syncing {:?} to {:?}", path, target_path);

        match fs::copy(path, target_path) {
            Ok(_) => {
                println!("Sync successful");
            },
            Err(error) => {
                println!("Error syncing: {}", error);
            }
        }
    }

    fn path_write(&self, path: PathBuf) {
        let target_path = self.build_target_path(&path);

        println!("Syncing {:?} to {:?}", path, target_path);

        match fs::copy(path, target_path) {
            Ok(_) => {
                println!("Sync successful");
            },
            Err(error) => {
                println!("Error syncing: {}", error);
            }
        }
    }

    fn path_remove(&self, path: PathBuf) {
        let target_path = self.build_target_path(&path);

        println!("deleteing {:?}", target_path);

        if target_path.exists() {
            if target_path.is_dir() {
                fs::remove_dir(target_path).unwrap();
            } else {
                fs::remove_file(target_path).unwrap();
            }
        }
    }

    fn path_chmod(&self, path: PathBuf) {
        let target_path = self.build_target_path(&path);

        println!("chmod {:?}", target_path);

        let permissions = path.metadata().unwrap().permissions();

        match fs::set_permissions(target_path, permissions) {
            Ok(_) => {
                println!("Chmod successful");
            },
            Err(error) => {
                println!("Error Chmod: {}", error);
            }
        }
    }

    fn path_rename(&self, from: PathBuf, to: PathBuf) {
        let from_target_path = self.build_target_path(&from);
        let to_target_path = self.build_target_path(&to);

        println!("renaming {:?} -> {:?}", from_target_path, to_target_path);


        match fs::rename(from_target_path, to_target_path) {
            Ok(_) => {
                println!("Chmod successful");
            },
            Err(error) => {
                println!("Error Chmod: {}", error);
            }
        }
    }

    fn build_target_path(&self, path: &PathBuf) -> PathBuf {
        let relative_path = path.strip_prefix(&self.source).unwrap();

        let mut sync_path = self.target.clone();

        sync_path.push_str("\\");
        sync_path.push_str(relative_path.to_str().unwrap());

        PathBuf::from(sync_path)
    }

    // fn sync_status(&self) -> f32 {
    //     let source_files = self.fetch_files(self.source.to_owned()).unwrap();
    //     let target_files = self.fetch_files(self.target.to_owned()).unwrap();

    //     let mut synced_files = vec![];

    //     for path in &source_files {
    //         for tpath in &target_files {
    //             if path.strip_prefix(&self.source).eq(&tpath.strip_prefix(&self.target)) {
    //                 synced_files.push(tpath)
    //             }
    //         }
    //     }

    //     let synced_file_count = synced_files.len() as f32;
    //     let source_file_count = source_files.len() as f32;

    //     (synced_file_count / source_file_count) * 100.0
    // }

    fn fetch_files(&self, path: String) -> io::Result<Vec<PathBuf>> {
        let mut entries = fs::read_dir(path)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;

        entries.sort();

        Ok(entries)
    }

    fn sync_files(&self) -> Result<(), std::io::Error> {
        let source_files = self.fetch_files(self.source.to_owned()).unwrap();
        let target_files = self.fetch_files(self.target.to_owned()).unwrap();

        // Wipe out extra target files
        'outer: for path in &target_files {
            for spath in &source_files {
                if path.strip_prefix(&self.target).eq(&spath.strip_prefix(&self.source)) {
                    println!("target exists in source: {:?}", path);
                    continue 'outer;
                }
            }

            println!("target doesn't exit in source, deleting... {:?}", path);

            if path.is_dir() {
                fs::remove_dir(path).unwrap();
            } else {
                fs::remove_file(path).unwrap();
            }
        }

        let target_files = self.fetch_files(self.target.to_owned()).unwrap();

        // Sync missing files
        'outer2: for path in &source_files {
            for tpath in &target_files {
                if path.strip_prefix(&self.source).eq(&tpath.strip_prefix(&self.target)) {
                    continue 'outer2;
                }
            }

            let mut tpath = self.target.clone();
            tpath.push_str("\\");
            tpath.push_str(path.strip_prefix(&self.source).unwrap().to_str().unwrap());

            if path.is_dir() {
                fs::create_dir(tpath).unwrap();
            } else {
                fs::copy(path, tpath).unwrap();
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Config {
    directories: Vec<Directory>,
}

fn main() {
    // Signal gets a value when the OS sent a INT or TERM signal.
    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);

    let config = parse_config();

    let pool = ThreadPool::new(config.directories.len());

    for directory in config.directories {
        pool.execute(move|| {
            directory.watch();
        });
    }

    signal.recv().unwrap();

    println!("Received quit...wrapping up.");

    pool.join();

    println!("Done...PEACE");
}

fn parse_config() -> Config {
    let config_str = fs::read_to_string("config.yaml").unwrap();
    let config_yaml = &YamlLoader::load_from_str(&config_str).unwrap()[0];

    let directories = parse_directories(config_yaml);

    Config { directories }
}

fn parse_directories(config_yaml: &yaml_rust::Yaml) -> Vec<Directory> {
    let directories_yaml = config_yaml["directories"].as_vec().unwrap();

    let mut dir_vec = vec![];

    for directory in directories_yaml {
        let source = directory["source"].as_str().unwrap().to_owned();
        let target = directory["target"].as_str().unwrap().to_owned();

        let dir = Directory { source, target };

        dir_vec.push(dir)
    }

    dir_vec
}
