// use chrono::{ DateTime, Local };
use rusqlite::{ params, Connection, OpenFlags, Result };
use serde::{ Deserialize, Serialize };
use std::path::Path;
use std::sync::Mutex;

// 数据库结构体
// pub struct DbHelper {
//     conn: Connection,
// }

pub struct DbHelper {
    conn: Mutex<Connection>, // 使用 Mutex 包装 Connection 以支持多线程
}

// 图片数据结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct Img {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub time: String,
    pub lat: f64,
    pub lng: f64,
}

// 实现 Img 结构体的方法
impl Img {
    // 定义 new 方法
    pub fn new() -> Self {
        Img {
            id: 0,
            name: String::new(),
            path: String::new(),
            time: String::new(),
            lat: 0.0,
            lng: 0.0,
        }
    }
}

impl DbHelper {
    // 初始化数据库连接
    pub fn new(db_path: &str) -> Result<Self> {
        let exists = Path::new(db_path).exists();
        let flags =
            OpenFlags::SQLITE_OPEN_READ_WRITE |
            OpenFlags::SQLITE_OPEN_CREATE |
            OpenFlags::SQLITE_OPEN_FULL_MUTEX;

        let conn = Connection::open_with_flags(db_path, flags)?;

        if !exists {
            conn.execute(
                "CREATE TABLE imgs (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        name TEXT NOT NULL,
                        path TEXT NOT NULL,
                        time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                        lat REAL,
                        lng REAL
                    )",
                []
            )?;
        }
        Ok(DbHelper {
            conn: Mutex::new(conn),
        })
        // Ok(DbHelper { conn })
    }

    // 插入数据
    // pub fn insert(&self, img: &Img) -> Result<()> {
    //     let conn = self.conn.lock().unwrap();
    //     // let tx = conn.transaction()?; // 开始事务
    //     conn.execute(
    //         "INSERT INTO imgs (name, path, time, lat, lon)
    //             VALUES (?1, ?2, ?3, ?4, ?5)",
    //         params![img.name, img.path, img.time, img.lat, img.lon],
    //     )?;
    //     // tx.commit()?; // 提交事务
    //     Ok(())
    // }

    pub fn insert_imgs(&self, imgs: &[Img]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap(); // 获取数据库连接
        let tx = conn.transaction()?; // 开始事务

        for img in imgs {
            tx.execute(
                "INSERT INTO imgs (name, path, time, lat, lng) 
                    VALUES (?1, ?2, ?3, ?4, ?5)",
                params![img.name, img.path, img.time, img.lat, img.lng]
            )?;
        }

        tx.commit()?; // 提交事务
        Ok(())
    }

    // 查询所有数据
    pub fn query_all(&self) -> Result<Vec<Img>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM imgs")?;
        let rows = stmt.query_map([], |row| {
            Ok(Img {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                time: row.get(3)?,
                lat: row.get(4)?,
                lng: row.get(5)?,
            })
        })?;

        let mut imgs = Vec::new();
        for row in rows {
            imgs.push(row?);
        }
        Ok(imgs)
    }

    // // 根据ID删除
    // pub fn delete_by_id(&self, id: i32) -> Result<()> {
    //     let conn = self.conn.lock().unwrap();
    //     conn.execute("DELETE FROM imgs WHERE id = ?1", params![id])?;
    //     Ok(())
    // }

    // // 更新经纬度
    // pub fn update_location(&self, id: i32, lan: f64, lon: f64) -> Result<()> {
    //     let conn = self.conn.lock().unwrap();
    //     conn.execute(
    //         "UPDATE imgs SET lat = ?1, lon = ?2 WHERE id = ?3",
    //         params![lan, lon, id],
    //     )?;
    //     Ok(())
    // }

    // 清空表（SQLite 不支持 TRUNCATE，使用 DELETE + VACUUM）
    pub fn truncate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM imgs", [])?;
        conn.execute("VACUUM", [])?;
        Ok(())
    }
}
