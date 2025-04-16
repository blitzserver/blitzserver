use std::io::Write;

use sqlx::{Postgres, Acquire, Connection, Executor};
use zip::{ZipWriter, write::FileOptions};

use crate::packets::{map_resp::{MapResp6, PlaneDef}};

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
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(MapSummary, r#"SELECT m.map_id, m.is_randommap, m.name, ARRAY_AGG(prov_count) "planes?: Vec<Option<i32>>" FROM maps_dom6 m LEFT JOIN map_planes mp ON mp.map_id=m.map_id WHERE is_randommap IS FALSE GROUP BY m.map_id"#).fetch_all(db).await
    }
}

impl Map {
    pub async fn get_mapresp<'q, D>(game_id: i32, db: D) -> Result<MapResp6, sqlx::error::Error> 
        where D: Executor<'q, Database = Postgres>
    {
        let mapstuff = sqlx::query!(r#"
            SELECT 
                pf.filename m_filename, mp.hash m_hash, length(pf.filebinary) m_length,
                if.filename i_filename, mi.hash i_hash, length(if.filebinary) i_length
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
                    map_len: ms.m_length.unwrap(),
                    map_hash: ms.m_hash,
                    d6m_filename: ms.i_filename.clone(),
                    img_len: ms.i_length.unwrap(),
                    img_hash: ms.i_hash,
                    plane_images: vec![]
                }
            }).collect::<Vec<_>>()
        })
    }
    pub async fn get_plane_image<'q, D>(map_id: i32, plane_id: i32, db: D) -> Result<PlaneImage, sqlx::error::Error>
        where D: Executor<'q, Database = Postgres>
    {
        let stuff = sqlx::query!(r#"SELECT mp.hash, f.filename, f.filebinary
        FROM maps_dom6 m
        INNER JOIN map_plane_images mp ON mp.map_id=m.map_id
        INNER JOIN files f ON f.id=mp.image_file_id
        WHERE m.map_id=$1
        ORDER BY map_file_id ASC LIMIT 1 OFFSET $2
        "#, map_id, plane_id as i64).fetch_one(db).await?;
        Ok(PlaneImage {
            filename: stuff.filename,
            hash: stuff.hash as u32,
            bytes: stuff.filebinary
        })
    }
    pub async fn get_archive<'q, D>(map_id: i32, db: D) -> Result<MapArchive, sqlx::error::Error>
        where D: Executor<'q, Database = Postgres>
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
        Ok(MapArchive {
            filename: format!("{}.zip", files[0].name),
            file_binary: buf,
        })
    }
    pub async fn insert<'q, E>(&self, db: E) -> Result<i32, sqlx::error::Error>
        where E: 'q + sqlx::Acquire<'q, Database = sqlx::Postgres>
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
        let planes = sqlx::query!(
            r#"
            WITH unnested AS (
                SELECT UNNEST($2::text[]) filename, UNNEST($3::BYTEA[]) contents, UNNEST($4::INTEGER[]) hash
            ),
            file AS (
                INSERT INTO files (filename, filebinary) SELECT filename, contents FROM unnested ON CONFLICT (filename,digest(filebinary, 'sha1'::text)) DO UPDATE SET filename=EXCLUDED.filename RETURNING *
            ),
            new_planes AS (
                INSERT INTO map_planes (map_id, map_file_id, hash) SELECT $1, f.id, u.hash
                FROM file f
                INNER JOIN unnested u ON u.filename=f.filename
                RETURNING *
            )
            SELECT filename, id
            FROM new_planes p 
            INNER JOIN file f ON f.id=p.map_file_id
            WHERE p.map_id=$1
            "#,
            map.map_id, 
            &self.planes.iter().map(|p| p.name.clone()).collect::<Vec<_>>()[..], 
            &self.planes.iter().map(|p| p.bytes.clone()).collect::<Vec<_>>()[..], 
            &self.planes.iter().map(|p| p.hash as i32).collect::<Vec<_>>()[..], 
        )
        .fetch_all(&mut *transaction)
        .await?;
        for plane in &self.planes {
            let image = sqlx::query!(r#"
                WITH unnested AS (
                    SELECT UNNEST($3::INTEGER[]) hash, UNNEST($4::TEXT[]) filename, UNNEST($5::BYTEA[]) contents
                ),
                file AS (
                    INSERT INTO files (filename, filebinary) 
                    SELECT filename, contents 
                    FROM unnested
                    ON CONFLICT (filename,digest(filebinary, 'sha1'::text)) DO UPDATE SET filename=EXCLUDED.filename RETURNING *
                )
                INSERT INTO map_plane_images (map_id, map_file_id, hash, image_file_id) 
                    SELECT $1, $2, hash, id 
                    FROM file f
                    INNER JOIN unnested u ON u.filename=f.filename
                    RETURNING *"#,
                map.map_id,
                planes.iter().find(|p| p.filename == plane.name).unwrap().id, 
                &plane.images.iter().map(|i| i.hash as i32).collect::<Vec<_>>()[..],
                &plane.images.iter().map(|i| i.filename.clone()).collect::<Vec<_>>()[..],
                &plane.images.iter().map(|i| i.bytes.clone()).collect::<Vec<_>>()[..],
                )
                .fetch_all(&mut *transaction)
                .await?;
        }
        
        transaction.commit().await?;

        Ok(map.map_id)
    }
}
