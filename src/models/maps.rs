use std::io::Write;

use sqlx::{Sqlite, Acquire, Connection, Executor};
use zip::{ZipArchive, ZipWriter, write::FileOptions};

use crate::packets::{map_resp::{MapResp6, PlaneDef}, MapResp};

#[derive(Debug)]
pub struct PlaneImage {
    pub filename: String,
    pub hash: u32,
    pub bytes: Vec<u8>,
}
#[derive(Debug)]
pub struct Plane {
    pub name: String,
    pub hash: u32,
    pub bytes: Vec<u8>,
    pub images: Vec<PlaneImage>,
}

#[derive(Debug)]
pub struct Map {
    pub name: String,
    pub is_randommap: bool,
    pub planes: Vec<Plane>,
}

#[derive(Debug)]
pub struct MapArchive {
    pub filename: String,
    pub file_binary: Vec<u8>
}

pub struct MapSummary {
    pub map_id: i32,
    pub name: String,
    pub is_randommap: bool,
    pub planes: Option<Vec<Option<i32>>>
}
impl MapSummary {
    pub async fn get_map_list<'q, D>(
        db: D
    ) -> Result<Vec<MapSummary>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Sqlite>
    {
        let results = sqlx::query!(r#"SELECT m.map_id, m.is_randommap, m.name,
            (SELECT json_group_array(mp.prov_count)
             FROM map_planes mp
             WHERE mp.map_id=m.map_id
             ORDER BY mp.map_file_id) as "planes?: String"
            FROM maps_dom6 m
            WHERE is_randommap IS FALSE"#)
            .fetch_all(db).await?;

        Ok(results.into_iter().map(|r| MapSummary {
            map_id: r.map_id.unwrap(),
            is_randommap: r.is_randommap.unwrap(),
            name: r.name.unwrap(),
            planes: r.planes.and_then(|p| serde_json::from_str(&p).ok())
        }).collect())
    }
}

impl Map {
    pub async fn get_mapresp<'q, D>(game_id: i32, db: D) -> Result<MapResp6, sqlx::error::Error> 
        where D: Executor<'q, Database = Sqlite>
    {
        let mapstuff = sqlx::query!(r#"
            SELECT 
                pf.filename m_filename, mp.hash m_hash, CAST(length(pf.filebinary) AS INTEGER) as "m_length: i32",
                if.filename i_filename, mi.hash i_hash, CAST(length(if.filebinary) AS INTEGER) as "i_length: i32"
            FROM games g
            INNER JOIN maps_dom6 dm ON dm.map_id=g.maps_dom6_id
            INNER JOIN map_planes mp ON mp.map_id=dm.map_id
            INNER JOIN files pf ON pf.id=mp.map_file_id
            INNER JOIN map_plane_images mi ON mp.map_file_id=mi.map_file_id AND mp.map_id=mi.map_id
            INNER JOIN files if ON if.id=mi.image_file_id
            WHERE g.id=$1
            ORDER BY pf.filename ASC
            "#
            , game_id).fetch_all(db).await?;

        return Ok(MapResp6 {
            planes: mapstuff.iter().map(|ms| {
                PlaneDef {
                    plane_name: "Unnamed Plane".to_owned(),
                    plane_file_name: ms.m_filename.clone(),
                    map_len: ms.m_length.unwrap() as i32,
                    map_hash: ms.m_hash as i32,
                    d6m_filename: ms.i_filename.clone(),
                    img_len: ms.i_length.unwrap() as i32,
                    img_hash: ms.i_hash as i32,
                    plane_images: vec![]
                }
            }).collect::<Vec<_>>()
        })
    }
    pub async fn get_plane_image<'q, D>(map_id: i64, plane_id: i32, db: D) -> Result<PlaneImage, sqlx::error::Error>
        where D: Executor<'q, Database = Sqlite>
    {
        let stuff = sqlx::query!(r#"SELECT mp.hash, f.filename, f.filebinary
        FROM maps_dom6 m
        INNER JOIN map_plane_images mp ON mp.map_id=m.map_id
        INNER JOIN files f ON f.id=mp.image_file_id
        WHERE m.map_id=$1
        ORDER BY map_file_id ASC LIMIT 1 OFFSET $2
        "#, map_id, plane_id as i64).fetch_one(db).await?;
        return Ok(PlaneImage {
            filename: stuff.filename,
            hash: stuff.hash as u32,
            bytes: stuff.filebinary
        })
    }
    pub async fn get_archive<'q, D>(map_id: i64, db: D) -> Result<MapArchive, sqlx::error::Error>
        where D: Executor<'q, Database = Sqlite>
    {
        let files = sqlx::query!(r#" SELECT 
                dm."name" "name",
                pf.filename m_filename, pf.filebinary m_filebinary,
                if.filename i_filename, if.filebinary i_filebinary
            FROM maps_dom6 dm 
            INNER JOIN map_planes mp ON mp.map_id=dm.map_id
            INNER JOIN files pf ON pf.id=mp.map_file_id
            INNER JOIN map_plane_images mi ON mp.map_file_id=mi.map_file_id AND mp.map_id=mi.map_id
            INNER JOIN files if ON if.id=mi.image_file_id
            WHERE dm.map_id=$1
            ORDER BY pf.filename ASC
            "#, map_id).fetch_all(db).await?;
        let mut buf: Vec<u8> = vec![];
        let mut zipfile = ZipWriter::new(std::io::Cursor::new(&mut buf));

        let options = FileOptions::default().large_file(true);
        for file in &files {
            zipfile.start_file::<_, (),_>(file.m_filename.to_owned(), options).unwrap();
            zipfile.write_all(&file.m_filebinary).unwrap();
            zipfile.start_file::<_, (), _>(file.i_filename.to_owned(), options).unwrap();
            zipfile.write_all(&file.i_filebinary).unwrap();
        }
        zipfile.finish().unwrap();
        return Ok(MapArchive {
            filename: format!("{}.zip", files[0].name),
            file_binary: buf,
        })
    }
    pub async fn insert<'q, E>(&self, mut db: E) -> Result<i64, sqlx::error::Error>
        where E: 'q + sqlx::Acquire<'q, Database = sqlx::Sqlite>
    {
        let mut transaction = db.begin().await?;
        let map = sqlx::query!(
            r#"
        INSERT INTO maps_dom6 ("name", "is_randommap") VALUES ($1, $2) RETURNING *
        "#,
            self.name, 
            self.is_randommap
        )
        .fetch_one(&mut *transaction)
        .await?;
        // Insert planes one by one
        let mut plane_file_ids = std::collections::HashMap::new();
        for plane in &self.planes {
            // Insert file
            let file = sqlx::query!(
                "INSERT INTO files (filename, filebinary) VALUES ($1, $2) RETURNING *",
                plane.name,
                plane.bytes
            )
            .fetch_one(&mut *transaction)
            .await?;

            // Insert map_plane
            sqlx::query!(
                "INSERT INTO map_planes (map_id, map_file_id, hash) VALUES ($1, $2, $3)",
                map.map_id,
                file.id,
                plane.hash as i32
            )
            .execute(&mut *transaction)
            .await?;

            plane_file_ids.insert(plane.name.clone(), file.id);
        }
        for plane in &self.planes {
            let plane_file_id = *plane_file_ids.get(&plane.name).unwrap();

            for image in &plane.images {
                // Insert image file
                let img_file = sqlx::query!(
                    "INSERT INTO files (filename, filebinary) VALUES ($1, $2) RETURNING *",
                    image.filename,
                    image.bytes
                )
                .fetch_one(&mut *transaction)
                .await?;

                // Insert map_plane_image
                sqlx::query!(
                    "INSERT INTO map_plane_images (map_id, map_file_id, hash, image_file_id) VALUES ($1, $2, $3, $4)",
                    map.map_id,
                    plane_file_id,
                    image.hash as i32,
                    img_file.id
                )
                .execute(&mut *transaction)
                .await?;
            }
        }
        
        transaction.commit().await?;

        return Ok(map.map_id);
    }
}
