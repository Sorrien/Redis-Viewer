pub(crate) mod redislogic {
    use redis::{Commands, Connection};
    use std::collections::HashMap;

    pub fn connect_redis(connection: &str) -> redis::RedisResult<Connection> {
        let client = redis::Client::open(connection)?;
        client.get_connection()
    }

    pub fn get_all_keys(redis: &mut redis::Connection) -> redis::RedisResult<Vec<String>> {
        let all_keys: Vec<String> = redis.keys("*")?;
        Ok(all_keys)
    }

    pub fn get_redis_value(
        redis: &mut redis::Connection,
        key: String,
    ) -> redis::RedisResult<String> {
        let value: String = redis.get(key)?;
        Ok(value)
    }

    pub fn set_redis_value(
        con: &mut redis::Connection,
        key: String,
        value: String,
    ) -> redis::RedisResult<()> {
        let _: () = con.set(key, value)?;
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
        name: String,
        sub_namespaces: HashMap<String, RedisNamespace>,
        keys: Vec<String>,
    }
}
