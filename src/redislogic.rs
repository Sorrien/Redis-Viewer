pub(crate) mod redislogic {
    use redis::{Commands, Connection, ConnectionAddr};
    use std::collections::HashMap;

    pub fn connect_redis(address: &str, port: u16, db: i64) -> redis::RedisResult<Connection> {
        let client = redis::Client::open(redis::ConnectionInfo {
            addr: Box::new(ConnectionAddr::Tcp(address.to_string(), port)),
            db,
            username: None,
            passwd: None,
        })?;
        client.get_connection()
    }

    pub fn get_all_keys(redis: &mut redis::Connection) -> redis::RedisResult<Vec<String>> {
        let all_keys: Vec<String> = redis.keys("*")?;
        Ok(all_keys)
    }

    pub fn get_redis_value(
        redis: &mut redis::Connection,
        key: &str,
    ) -> redis::RedisResult<RedisValue> {
        let key_type: String = redis::cmd("TYPE")
            .arg(key)
            .query(redis)
            .expect("failed to get key type");
        let value: Result<RedisValue, redis::RedisError> = match key_type.as_str() {
            "string" => {
                let v: String = redis.get(key)?;
                Ok(RedisValue::String(v))
            }
            "list" => {
                let v: Vec<String> = redis.lrange(key, 0, -1)?;
                Ok(RedisValue::List(v))
            }
            "set" => {
                let v: Vec<String> = redis.smembers(key)?;
                Ok(RedisValue::Set(v))
            }
            "zset" => {
                let v: Vec<(String, String)> = redis.zrangebyscore(key, "-inf", "+inf")?;
                Ok(RedisValue::ZSet(v))
            }
            "hash" => {
                let v: HashMap<String, String> = redis.hgetall(key)?;
                Ok(RedisValue::Hash(v))
            }
            _ => Ok(RedisValue::Null),
        };
        Ok(value?)
    }

    pub fn set_redis_value(
        con: &mut redis::Connection,
        key: String,
        value: String,
    ) -> redis::RedisResult<()> {
        let _: () = con.set(key, value)?;
        Ok(())
    }

    pub fn delete_redis_key(con: &mut redis::Connection, key: String) -> redis::RedisResult<()> {
        let _: () = con.del(key)?;
        Ok(())
    }

    pub fn convert_keys_to_namespaces(keys: &Vec<String>) -> HashMap<String, RedisNamespace> {
        let mut namespaces = HashMap::<String, RedisNamespace>::new();

        let mut empty_namespace = RedisNamespace {
            name: "".into(),
            sub_namespaces: HashMap::<String, RedisNamespace>::new(),
            keys: Vec::<String>::new(),
        };

        for key in keys {
            let parts: Vec<&str> = key.split(":").collect();
            if parts.len() == 1 {
                empty_namespace.keys.push(key.clone());
            } else {
                add_key_to_namespaces(parts, &mut namespaces, 0);
            }
        }
        namespaces.insert("".into(), empty_namespace);
        namespaces
    }

    pub fn add_key_to_namespaces(
        parts: Vec<&str>,
        current_namespace: &mut HashMap<String, RedisNamespace>,
        part_index: usize,
    ) {
        let part = parts[part_index];
        let result = current_namespace.get_mut(part);

        let next_namespace = match result {
            Some(namespace) => namespace,
            None => {
                current_namespace.insert(
                    part.into(),
                    RedisNamespace {
                        name: part.into(),
                        sub_namespaces: HashMap::<String, RedisNamespace>::new(),
                        keys: Vec::<String>::new(),
                    },
                );
                current_namespace.get_mut(part).unwrap()
            }
        };

        if part_index == parts.len() - 1 {
            next_namespace.keys.push(parts.join(":"));
        } else {
            add_key_to_namespaces(parts, &mut next_namespace.sub_namespaces, part_index + 1);
        }
    }

    pub struct RedisNamespace {
        pub name: String,
        pub sub_namespaces: HashMap<String, RedisNamespace>,
        pub keys: Vec<String>,
    }

    pub enum RedisValue {
        String(String),
        List(Vec<String>),
        Set(Vec<String>),
        ZSet(Vec<(String, String)>),
        Hash(HashMap<String, String>),
        Null,
    }
}
