use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::BufReader,
    io::{BufRead, Write},
    str::FromStr,
    sync::{Arc, RwLock},
};

use actix_web::web;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::{
    file_utils::{file_exists, make_dirs_and_write},
    shared_data::Sharable,
    Error, Result,
};

pub fn hash_password<T>(plain_password: T) -> String
where
    T: AsRef<[u8]>,
{
    // hash_encoded returns a Result<String>, but as far as I can tell the only
    // failure condition is an unallowed salt len

    let mut salt = [0u8; 16];

    rand::thread_rng().fill_bytes(&mut salt);
    return argon2::hash_encoded(plain_password.as_ref(), &salt, &argon2::Config::default()).unwrap();
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AcctType {
    Admin,
    User,
}

impl FromStr for AcctType {
    type Err = ();

    fn from_str(input: &str) -> Result<AcctType, Self::Err> {
        match input {
            "Admin" => Ok(AcctType::Admin),
            "User" => Ok(AcctType::User),
            _ => Err(()),
        }
    }
}

pub struct User {
    name: String,
    acct_type: AcctType,
    hashed_password: String,
    paths: Vec<String>,
    path_patterns: Vec<glob::Pattern>,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        // The username should be unique since the only time a User{} gets
        // created is immediately after checking for an identical name, and
        // that name is the key in the hashmap, but it never hurts to be
        // safe and check everything. This doesn't get called often.
        self.name == other.name
            && self.acct_type == other.acct_type
            && self.hashed_password == other.hashed_password
            && self.paths == other.paths
            && self.path_patterns == other.path_patterns
    }
}

impl User {
    pub fn new(
        name: &String,
        hashed_pass: &String,
        allowed_paths: &Vec<String>,
        acct_type: AcctType,
    ) -> Result<Self> {
        let patterns = match User::parse_paths(&allowed_paths) {
            Ok(p) => p,
            Err(e) => return Err(e),
        };

        return Ok(User {
            name: name.to_owned(),
            hashed_password: hashed_pass.to_owned(),
            paths: allowed_paths.to_owned(),
            path_patterns: patterns,
            acct_type,
        });
    }

    pub fn update_info(
        &mut self,
        plain_password: Option<String>,
        paths: Option<Vec<String>>,
        acct_type: Option<AcctType>,
    ) -> Result<()> {
        // Validate everything first
        let mut hashed_pass: Option<String> = None;
        match plain_password {
            Some(p) => match UserDB::validate_password(&p) {
                Ok(_) => hashed_pass = Some(hash_password(p)),
                Err(e) => return Err(e),
            },
            None => {}
        }

        let mut parsed_paths: Option<Vec<glob::Pattern>> = None;
        match &paths {
            Some(p) => match User::parse_paths(&p) {
                Ok(parsed) => parsed_paths = Some(parsed),
                Err(e) => return Err(e),
            },
            None => {}
        }

        // Assign new values
        match hashed_pass {
            Some(h) => self.hashed_password = h,
            None => {}
        }

        match parsed_paths {
            Some(p) => {
                self.path_patterns = p;
                self.paths = paths.unwrap();
            }
            None => {}
        }

        match acct_type {
            Some(c) => self.acct_type = c,
            None => {}
        }

        return Ok(());
    }

    fn parse_paths(path_strings: &Vec<String>) -> Result<Vec<glob::Pattern>> {
        let mut patterns = Vec::<glob::Pattern>::new();

        for s in path_strings {
            match glob::Pattern::new(s) {
                Ok(p) => patterns.push(p),
                Err(e) => {
                    return Err(Error::new(format!("Invalid glob pattern {}: {}", s, e)));
                }
            }
        }
        return Ok(patterns);
    }

    pub fn path_allowed(&self, path: &str) -> bool {
        for pattern in &self.path_patterns {
            if pattern.matches(path) {
                return true;
            }
        }
        return false;
    }

    pub fn get_name(&self) -> &String {
        return &self.name;
    }

    pub fn get_type(&self) -> &AcctType {
        return &self.acct_type;
    }

    fn to_line(&self) -> String {
        return format!(
            "{}:{}:{}:{:#?}",
            self.name,
            self.hashed_password,
            self.paths.join(","),
            self.acct_type
        );
    }
}

// Serializable User without password
#[derive(Serialize, Deserialize)]
pub struct SafeSerializableUser {
    pub name: String,
    pub acct_type: AcctType,
    pub paths: Vec<String>,
}

impl From<&Arc<RwLock<User>>> for SafeSerializableUser {
    fn from(u: &Arc<RwLock<User>>) -> Self {
        let u = u.read().unwrap();
        return Self {
            name: u.name.to_owned(),
            acct_type: u.acct_type,
            paths: u.paths.to_owned(),
        };
    }
}

pub struct UserDB {
    users: HashMap<String, Arc<RwLock<User>>>, // K: Name, V: User
    filepath: String,
}

impl UserDB {
    pub fn new(filepath: &String) -> Result<Self> {
        let mut um = UserDB {
            users: HashMap::new(),
            filepath: filepath.clone(),
        };

        return match um.parse_file(filepath) {
            Ok(_) => Ok(um),
            Err(e) => Err(e),
        };
    }

    fn parse_file(&mut self, filepath: &String) -> Result<()> {
        tracing::info!("Loading {}", filepath);
        if !file_exists(filepath) {
            match File::create(filepath) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    tracing::error!("Could not create file: {}", e);
                    return Err(Error::convert(e));
                }
            }
        };

        let file = match File::open(filepath) {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("Could not open file: {}", e);
                return Err(Error::convert(e));
            }
        };

        let reader = BufReader::new(file);
        for (i, line) in reader.lines().enumerate() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    self.users.clear();
                    tracing::error!("Corrupt user db: {}", e.to_string());
                    return Err(Error::convert(e));
                }
            };
            let parts: Vec<String> = line.split(':').map(|s| s.to_string()).collect();
            if parts.len() != 4 {
                tracing::warn!("Invalid entry on line {} of {}", i, filepath);
                continue;
            }

            if self.users.contains_key(&parts[0]) {
                tracing::warn!(
                    "Duplicate user `{}` on line {}; this line will be ignored.",
                    parts[0],
                    i
                );
            }

            let acct_type = match AcctType::from_str(&parts[3]) {
                Ok(r) => r,
                Err(_) => {
                    tracing::warn!(
                        "Invalid acct type `{}` on line {}; this line will be ignored",
                        parts[3],
                        i
                    );
                    continue;
                }
            };

            let usr = match User::new(
                &parts[0],
                &parts[1],
                &parts[2].split(',').map(|s| s.to_string()).collect(),
                acct_type,
            ) {
                Ok(u) => u,
                Err(e) => return Err(e),
            };

            self.users.insert(parts[0].to_owned(), Arc::new(RwLock::new(usr)));
        }
        return Ok(());
    }

    pub fn count(&self) -> usize {
        return self.users.len();
    }

    /// Adds a new unique user. Will return an error if that user name
    /// already exists in the db
    pub fn add_user(
        &mut self,
        name: &String,
        plain_password: &String,
        allowed_paths: &Vec<String>,
        acct_type: AcctType,
    ) -> Result<()> {
        match self.validate_username(name, true) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        match Self::validate_password(plain_password) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let user = match User::new(name, &hash_password(plain_password), allowed_paths, acct_type) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };

        match self.append_to_file(&user) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        self.users.insert(name.to_owned(), Arc::new(RwLock::new(user)));

        return Ok(());
    }

    /// Checks if username is suitable for use (eg length, invalid chars).
    /// check_existing also checks for collisions against existing names
    pub fn validate_username(&self, name: &String, check_existing: bool) -> Result<()> {
        if check_existing && self.users.contains_key(name) {
            return crate::err!("User `{}` already exists", name);
        };

        if name.contains(":") {
            return crate::err!("Username cannot contain `:`");
        };

        if name.len() == 0 {
            return crate::err!("User name may not be empty");
        }

        if name.len() > 72 {
            return crate::err!("User name too long (maximum 72 characters)");
        }

        return Ok(());
    }

    pub fn validate_password(plain_password: &String) -> Result<()> {
        if plain_password.len() < 8 {
            return crate::err!("Password too short (minimum 8 characters)");
        }

        // Sounds like a good enough limit
        if plain_password.len() > 72 {
            return crate::err!("Password too long (maximum 72 characters)");
        }

        return Ok(());
    }

    fn append_to_file(&self, user: &User) -> Result<()> {
        let mut file = match OpenOptions::new().write(true).append(true).open(self.filepath.as_str()) {
            Ok(f) => f,
            Err(e) => return Err(Error::convert(e)),
        };

        return match writeln!(file, "{}", user.to_line()) {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::convert(e)),
        };
    }

    /// Gets a user from the database with a matching name and password
    /// Should be used to verify a user login
    pub fn verify_credentials(&self, name: &String, plain_pass: &String) -> Option<&Arc<RwLock<User>>> {
        let user = match self.users.get(name) {
            Some(u) => u,
            None => return None,
        };

        let is_match = match argon2::verify_encoded(&user.read().unwrap().hashed_password, plain_pass.as_bytes()) {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Could not compare password and hash: {}", e);
                return None;
            }
        };

        if is_match {
            return Some(user);
        } else {
            return None;
        };
    }

    pub fn get(&self, name: &String) -> Option<&Arc<RwLock<User>>> {
        return self.users.get(name);
    }

    // Safe prevents sensitive data like hashed passwords from being serialized
    pub fn list_safe(&self) -> Vec<SafeSerializableUser> {
        let usrs: Vec<SafeSerializableUser> =
            self.users.iter().map(|(_, v)| SafeSerializableUser::from(v)).collect();
        return usrs;
    }

    pub fn write_to_file(&mut self) -> Result<()> {
        let mut lines: Vec<String> = Vec::with_capacity(self.users.len());

        for user in self.users.values() {
            lines.push(user.read().unwrap().to_line());
        }

        return make_dirs_and_write(self.filepath.as_str(), lines.join("\n"));
    }

    /// Removes user from in-memory database and writes database contents to file
    /// Any error will result from writing the database to disk
    pub fn remove(&mut self, name: &String) -> Result<()> {
        let r = match self.users.remove(name) {
            Some(_) => true,
            None => false,
        };

        if r {
            return self.write_to_file();
        }

        return Ok(());
    }
}

impl Sharable for UserDB {
    type Shared = RwLock<Self>;
    fn to_sharable(self) -> web::Data<Self::Shared> {
        return web::Data::new(RwLock::new(self));
    }
}

#[cfg(test)]
mod user_db_tests {
    use std::fs;

    use crate::test_utils::make_tmp_file;

    use super::*;
    const NAME: &str = "TestUser";
    const PASS: &str = "TestPass";

    #[test]
    fn add_user() {
        let tmp = make_tmp_file();
        let mut user_db = UserDB::new(&tmp).unwrap();
        let added = user_db.add_user(
            &NAME.to_string(),
            &PASS.to_string(),
            &vec!["*".to_string()],
            AcctType::Admin,
        );
        assert!(added.is_ok());
        assert!(fs::read_to_string(&tmp).unwrap().starts_with(NAME));

        _ = fs::remove_file(&tmp);
        let added = user_db.add_user(
            &"a_new_name".to_string(),
            &"a_new_pass".to_string(),
            &vec!["*".to_string()],
            AcctType::User,
        );
        assert!(added.is_err());
    }

    #[test]
    fn check_creds() {
        let tmp = make_tmp_file();
        let mut user_db = UserDB::new(&tmp).unwrap();
        _ = user_db.add_user(
            &NAME.to_string(),
            &PASS.to_string(),
            &vec!["*".to_string()],
            AcctType::Admin,
        );

        assert!(user_db
            .verify_credentials(&NAME.to_string(), &PASS.to_string())
            .is_some());

        assert!(user_db
            .verify_credentials(&NAME.to_string(), &"not_a_pass".to_string())
            .is_none());
        assert!(user_db
            .verify_credentials(&"not_a_user".to_string(), &PASS.to_string())
            .is_none());
    }

    #[test]
    fn invalid_name_pass() {
        let tmp = make_tmp_file();
        let mut user_db = UserDB::new(&tmp).unwrap();
        _ = user_db.add_user(
            &NAME.to_string(),
            &PASS.to_string(),
            &vec!["*".to_string()],
            AcctType::Admin,
        );
        assert!(user_db
            .add_user(
                &NAME.to_string(),
                &PASS.to_string(),
                &vec!["*".to_string()],
                AcctType::Admin
            )
            .is_err());
        assert!(user_db
            .add_user(
                &"".to_string(),
                &PASS.to_string(),
                &vec!["*".to_string()],
                AcctType::Admin
            )
            .is_err());
        assert!(user_db
            .add_user(
                &"test:user".to_string(),
                &PASS.to_string(),
                &vec!["*".to_string()],
                AcctType::Admin
            )
            .is_err());
        assert!(user_db
            .add_user(
                &"bad_pass".to_string(),
                &"1234567".to_string(),
                &vec!["*".to_string()],
                AcctType::Admin
            )
            .is_err());
        assert!(user_db
            .add_user(
                &"1234567891123456789212345678931234567894123456789512345678961234567897123".to_string(),
                &PASS.to_string(),
                &vec!["*".to_string()],
                AcctType::Admin
            )
            .is_err());
    }

    #[test]
    fn read_from_file() {
        let tmp = make_tmp_file();
        let mut user_db = UserDB::new(&tmp).unwrap();
        _ = user_db.add_user(
            &NAME.to_string(),
            &PASS.to_string(),
            &vec!["*".to_string()],
            AcctType::Admin,
        );

        let user_db = UserDB::new(&tmp).unwrap();

        let content = fs::read_to_string(tmp).unwrap();
        let lines: Vec<&str> = content.split('\n').filter(|l| l != &"").collect();
        assert_eq!(lines.len(), 1);
        assert!(user_db
            .verify_credentials(&NAME.to_string(), &PASS.to_string())
            .is_some());
    }

    #[test]
    fn duplicate_users() {
        let tmp = make_tmp_file();

        _ = fs::write(
            &tmp,
            "username:password:*:Admin\nusername:_password:_*:User".to_string(),
        );
        let user_db = UserDB::new(&tmp).unwrap();
        assert_eq!(user_db.count(), 1);
    }

    #[test]
    fn new_file() {
        let tmp_name = make_tmp_file();
        _ = fs::remove_file(&tmp_name);
        let mut user_db = UserDB::new(&tmp_name).unwrap();
        _ = user_db.add_user(
            &NAME.to_string(),
            &PASS.to_string(),
            &vec!["*".to_string()],
            AcctType::Admin,
        );

        user_db = UserDB::new(&tmp_name).unwrap();

        let content = fs::read_to_string(tmp_name).unwrap();
        let lines: Vec<&str> = content.split('\n').filter(|l| l != &"").collect();
        assert_eq!(lines.len(), 1);
        assert!(user_db
            .verify_credentials(&NAME.to_string(), &PASS.to_string())
            .is_some());

        let tmp_name = String::from("invalid_file_path!)@(#*$&%^|}{[]';:?/.<>,");
        let err_user_db = UserDB::new(&tmp_name);
        assert!(err_user_db.is_err());

        let readonly_tmp_name = make_tmp_file();
        let mut perms = fs::metadata(&readonly_tmp_name).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&readonly_tmp_name, perms).unwrap();
        let err_fs = UserDB::new(&tmp_name);
        assert!(err_fs.is_err());
    }

    #[test]
    fn read_bad_file_content() {
        let tmp = make_tmp_file();

        _ = fs::write(&tmp, "\nusername:password".to_string());
        let user_db = UserDB::new(&tmp).unwrap();
        assert_eq!(user_db.count(), 0);

        _ = fs::write(&tmp, "username:pass:word".to_string());
        let user_db = UserDB::new(&tmp).unwrap();
        assert_eq!(user_db.count(), 0);

        _ = fs::write(&tmp, "username:password:*:Null".to_string());
        let user_db = UserDB::new(&tmp).unwrap();
        assert_eq!(user_db.count(), 0);
    }

    #[test]
    fn list_safe() {
        let tmp = make_tmp_file();
        let mut user_db = UserDB::new(&tmp).unwrap();
        _ = user_db.add_user(
            &NAME.to_string(),
            &PASS.to_string(),
            &vec!["*".to_string()],
            AcctType::Admin,
        );
        _ = user_db.add_user(
            &"another_user".to_string(),
            &PASS.to_string(),
            &vec!["*".to_string()],
            AcctType::Admin,
        );

        let mut list = user_db.list_safe();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, NAME.to_string());
        assert_eq!(list[1].name, "another_user".to_string());
    }

    #[test]
    fn remove() {
        let tmp = make_tmp_file();
        _ = fs::write(&tmp, "username:password:*:Admin".to_string());
        let mut user_db = UserDB::new(&tmp).unwrap();
        assert_eq!(user_db.count(), 1);

        user_db
            .remove(&"username".to_string())
            .expect("Unable to write db to file");
        let user_db = UserDB::new(&tmp).unwrap();
        assert_eq!(user_db.count(), 0);
    }
}

#[cfg(test)]
mod user_tests {
    use std::fs;

    use super::*;
    use crate::test_utils::*;

    const NAME: &str = "TestUser";
    const PASS: &str = "TestPass";

    #[test]
    fn path_allowed() {
        let usr = User::new(
            &NAME.to_string(),
            &PASS.to_string(),
            &vec!["/rootonly".to_string(), "/a/*/b".to_string(), "/any/**".to_string()],
            AcctType::Admin,
        )
        .unwrap();

        assert!(usr.path_allowed("/rootonly"));
        assert!(!usr.path_allowed("/rootonly/"));

        assert!(usr.path_allowed("/a/wc/b"));
        assert!(!usr.path_allowed("/a/b"));
        assert!(!usr.path_allowed("/a/wc/b/c"));

        assert!(usr.path_allowed("/any/foo.bar"));
        assert!(usr.path_allowed("/any/1/foo.bar"));
        assert!(usr.path_allowed("/any/2/3/foo.bar"));
        assert!(!usr.path_allowed("/any"));
    }

    #[test]
    fn update_info() {
        let tmp = make_tmp_file();
        let mut user_db = UserDB::new(&tmp).unwrap();
        let added = user_db.add_user(
            &NAME.to_string(),
            &PASS.to_string(),
            &vec!["*".to_string()],
            AcctType::Admin,
        );
        assert!(added.is_ok());

        let mut user_w = user_db.get(&NAME.to_string()).unwrap().write().unwrap();
        let replacement_paths = vec![String::from("foo"), String::from("bar")];
        let res = user_w.update_info(
            Some("a_new_password".to_string()),
            Some(replacement_paths.to_owned()),
            Some(AcctType::User),
        );
        assert!(res.is_ok());

        assert_eq!(user_w.name, NAME.to_string());
        assert_eq!(user_w.paths, replacement_paths);
        assert_eq!(user_w.acct_type, AcctType::User);

        drop(user_w);
        let res = user_db.write_to_file();
        assert!(res.is_ok());

        let user_db = UserDB::new(&tmp).unwrap();
        let user = user_db.get(&NAME.to_string()).unwrap().read().unwrap();
        assert_eq!(user.name, NAME.to_string());
        assert!(argon2::verify_encoded(&user.hashed_password, b"a_new_password").unwrap());
        assert_eq!(user.paths, replacement_paths);
        assert_eq!(user.acct_type, AcctType::User);
    }

    #[test]
    fn to_line() {
        let line = "name:pw:*:User".to_string();
        let tmp = make_tmp_file();
        _ = fs::write(&tmp, &line);
        let user_db = UserDB::new(&tmp).unwrap();
        assert_eq!(user_db.count(), 1);
        let usr = user_db.get(&"name".to_string()).unwrap().read().unwrap();
        assert_eq!(usr.to_line(), line);
    }

    #[test]
    /// This is really just here so the coverage for this file doesn't look terrible
    fn get_fields() {
        let usr = User::new(
            &NAME.to_string(),
            &PASS.to_string(),
            &vec!["/rootonly".to_string(), "/a/*/b".to_string(), "/any/**".to_string()],
            AcctType::Admin,
        )
        .unwrap();

        assert_eq!(usr.get_name(), &NAME);
        assert_eq!(usr.get_type(), &AcctType::Admin);
    }
}
