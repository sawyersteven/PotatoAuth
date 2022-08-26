use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use actix_web::{
    cookie::{
        time::{Duration, OffsetDateTime},
        Cookie, CookieBuilder, SameSite,
    },
    HttpRequest,
};
use uuid::Uuid;

use crate::{routes::SESSION_NAME, shared_data::Sharable, userdb::User};

pub struct Session {
    user: Arc<RwLock<User>>,
    exp: OffsetDateTime,
    id: String,
}

impl Session {
    pub fn new(user: &Arc<RwLock<User>>, id: &String, ttl: Duration) -> Self {
        return Session {
            user: user.clone(),
            exp: OffsetDateTime::now_utc() + ttl,
            id: id.to_owned(),
        };
    }

    /// Returns a clone of the User assigned to this session
    pub fn get_user(&self) -> Arc<RwLock<User>> {
        return self.user.clone();
    }

    /// Generate cookie from session id
    /// The cookie will contain only one value - the key used to access the
    /// session in the store. No data will be sent to the client other than
    /// this key, so nothing needs to be signed or otherwise obfuscated.
    pub fn cookie(&self) -> Cookie {
        return CookieBuilder::new(SESSION_NAME, &self.id)
            .same_site(SameSite::Strict)
            .finish();
    }
}

pub struct SessionStore {
    ttl: Duration,
    sessions: HashMap<String, Session>,
}

impl SessionStore {
    pub fn new(session_ttl: Duration) -> Self {
        return SessionStore {
            ttl: session_ttl,
            sessions: HashMap::new(),
        };
    }

    /// Creates a new session with a unique key and assigns a clone of the
    /// provided User to the session data. Returns a borrow of the newly
    /// created session, which contains its id.
    pub fn new_session(&mut self, user: &Arc<RwLock<User>>) -> &Session {
        let key = Uuid::new_v4().to_string();

        self.sessions
            .insert(key.to_owned(), Session::new(&user, &key, self.ttl));
        let s = self.sessions.get(&key).unwrap();

        return s;
    }

    pub fn remove_id(&mut self, id: &String) {
        self.sessions.remove(id);
    }

    pub fn remove_for_user(&mut self, user: &Arc<RwLock<User>>) {
        let username = user.read().unwrap().get_name().to_owned();
        self.sessions
            .retain(|_, sess| sess.user.read().unwrap().get_name() != &username);
    }

    /// Gets session using cookie header in req.
    pub fn get_from_request(&mut self, req: &HttpRequest) -> Option<&Session> {
        let id = req.cookie(SESSION_NAME)?.value().to_string();
        return self.get(&id);
    }

    /// Gets a session that matches the provided id.
    /// Will return None if no such session exists or if the session has
    /// expired. An expired session will be removed from the store.
    /// A returned session will have its expiration updated.
    fn get(&mut self, id: &String) -> Option<&Session> {
        let sess = match self.sessions.get(id) {
            Some(s) => s,
            None => return None,
        };

        let now = OffsetDateTime::now_utc();

        if sess.exp < now {
            self.sessions.remove(id);
            return None;
        }

        let mut sess = self.sessions.get_mut(id).unwrap();
        sess.exp = now + self.ttl;
        return Some(sess);
    }
}

impl Sharable for SessionStore {
    type Shared = RwLock<SessionStore>;

    fn to_sharable(self) -> actix_web::web::Data<Self::Shared> {
        return actix_web::web::Data::new(RwLock::new(self));
    }
}

#[cfg(test)]
mod session_store_tests {
    use crate::userdb::AcctType;

    use super::*;

    #[test]
    fn new_session() {
        let mut session_store = SessionStore::new(Duration::seconds(60));

        let usr = User::new(
            &"user".to_string(),
            &"password".to_string(),
            &vec!["*".to_string()],
            AcctType::Admin,
        )
        .unwrap();
        let usr = Arc::new(RwLock::new(usr));

        let sess = session_store.new_session(&usr);

        assert!(sess.get_user().read().unwrap().eq(&usr.read().unwrap()));
    }

    #[test]
    fn remove() {
        let mut session_store = SessionStore::new(Duration::seconds(60));

        let usr = Arc::new(RwLock::new(
            User::new(
                &"user".to_string(),
                &"password".to_string(),
                &vec!["*".to_string()],
                AcctType::Admin,
            )
            .unwrap(),
        ));
        let id = session_store.new_session(&usr).id.to_owned();

        let usr = Arc::new(RwLock::new(
            User::new(
                &"user2".to_string(),
                &"password".to_string(),
                &vec!["*".to_string()],
                AcctType::Admin,
            )
            .unwrap(),
        ));
        let _id2 = session_store.new_session(&usr).id.to_owned();

        assert_eq!(session_store.sessions.len(), 2);

        session_store.remove_id(&id);
        assert_eq!(session_store.sessions.len(), 1);

        session_store.remove_id(&id);
        assert_eq!(session_store.sessions.len(), 1);

        assert!(session_store.get(&id).is_none());
    }

    #[tokio::test]
    async fn expiration() {
        let mut session_store = SessionStore::new(Duration::milliseconds(100));

        let usr = User::new(
            &"user".to_string(),
            &"password".to_string(),
            &vec!["*".to_string()],
            AcctType::Admin,
        )
        .unwrap();
        let usr = Arc::new(RwLock::new(usr));
        let id = session_store.new_session(&usr).id.to_owned();

        assert!(session_store.get(&id).is_some());
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(session_store.get(&id).is_some());
        tokio::time::sleep(std::time::Duration::from_millis(110)).await;
        assert!(session_store.get(&id).is_none());
    }
}
