mod domain;

use sqlx::PgPool;

struct Rating {
    id: i32,
    username: String,
    game_name: String,
    rating: Option<f32>,
    timestamp: i32,
}

struct RatingRepository {
    db: Database,
}

impl RatingRepository {
    fn new(db: PgPool) -> RatingRepository {
        RatingRepository { db }
    }

    fn get_ratings(&self, username: i32) -> Result<Vec<Rating>, String> {
        let mut ratings = Vec::new();
        let mut conn = self.db.get_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM ratings WHERE username = ?")?;
        let mut rows = stmt.query(params![username])?;
        while let Some(row) = rows.next()? {
            ratings.push(Rating {
                id: row.get(0)?,
                username: row.get(1)?,
                game_name: row.get(2)?,
                rating: row.get(3)?,
                timestamp: row.get(4)?,
            });
        }
        Ok(ratings)
    }

    fn add_rating(&self, rating: Rating) -> Result<(), String> {
        let mut conn = self.db.get_connection()?;
        let mut stmt = conn.prepare("INSERT INTO ratings (username, game_name, rating, timestamp) VALUES (?, ?, ?, ?)")?;
        stmt.execute(params![rating.username, rating.game_name, rating.rating, rating.timestamp])?;
        Ok(())
    }
}

struct RatingService {
    repo: RatingRepository,
}

impl RatingService {
    fn new(repo: RatingRepository) -> RatingService {
        RatingService { repo }
    }

    fn get_ratings(&self, username: i32) -> Result<Vec<Rating>, String> {
        self.repo.get_ratings(username)
    }

    fn add_rating(&self, rating: Rating) -> Result<(), String> {
        self.repo.add_rating(rating)
    }
}