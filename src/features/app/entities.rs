use sqlx::FromRow;

use super::interface::protobuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RatingsBand {
    VeryGood = 0,
    Good = 1,
    Neutral = 2,
    Poor = 3,
    VeryPoor = 4,
    InsufficientVotes = 5,
}

#[derive(Debug, Clone, FromRow)]
pub struct Rating {
    pub snap_id: String,
    pub total_votes: u64,
    pub ratings_band: RatingsBand,
}

impl Rating {
    pub fn new(snap_id: String, votes: Vec<Vote>) -> Self {
        let total_votes = votes.len();
        let ratings_band = calculate_band(votes);
        Self {
            snap_id,
            total_votes: total_votes as u64,
            ratings_band,
        }
    }

    pub(crate) fn into_dto(self) -> protobuf::Rating {
        protobuf::Rating {
            snap_id: self.snap_id,
            total_votes: self.total_votes,
            ratings_band: self.ratings_band as i32,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Vote {
    pub vote_up: bool,
}

fn calculate_band(votes: Vec<Vote>) -> RatingsBand {
    let total_ratings = votes.len();
    if total_ratings < 25 {
        return RatingsBand::InsufficientVotes;
    }
    let positive_ratings = votes
        .into_iter()
        .filter(|vote| vote.vote_up)
        .collect::<Vec<Vote>>()
        .len();
    let adjusted_ratio = confidence_interval_lower_bound(positive_ratings, total_ratings);

    if adjusted_ratio > 0.8 {
        return RatingsBand::VeryGood;
    } else if adjusted_ratio <= 0.8 && adjusted_ratio > 0.55 {
        return RatingsBand::Good;
    } else if adjusted_ratio <= 0.55 && adjusted_ratio > 0.45 {
        return RatingsBand::Neutral;
    } else if adjusted_ratio <= 0.45 && adjusted_ratio > 0.2 {
        return RatingsBand::Poor;
    } else {
        return RatingsBand::VeryPoor;
    }
}

fn confidence_interval_lower_bound(positve_ratings: usize, total_ratings: usize) -> f64 {
    if total_ratings == 0 {
        return 0.0;
    }
    let z: f64 = 1.96; // hardcoded for a ~95% confidence
    let n: f64 = total_ratings as f64;
    let phat = positve_ratings as f64 / n;
    ((phat + (z * z) / (2.0 * n))
        - z * f64::sqrt((phat * (1.0 - phat) + ((z * z) / (4.0 * n))) / n))
        / (1.0 + (z * z) / n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        let lower_bound = confidence_interval_lower_bound(0, 0);
        assert_eq!(
            lower_bound, 0.0,
            "Lower bound should be 0.0 when there are 0 votes"
        );
    }

    #[test]
    fn test_lb_approaches_true_ratio() {
        let ratio: f64 = 0.9;
        let mut last_lower_bound = 0.0;

        for total_ratings in (100..1000).step_by(100) {
            let positive_ratings = (total_ratings as f64 * ratio).round() as usize;
            let new_lower_bound = confidence_interval_lower_bound(positive_ratings, total_ratings);
            let raw_positive_ratio = positive_ratings as f64 / total_ratings as f64;

            // As the total ratings increase, the new lower bound should be closer to the raw positive ratio.
            assert!(
                (raw_positive_ratio - new_lower_bound).abs() <= (raw_positive_ratio - last_lower_bound).abs(),
                "As the number of votes goes up, the lower bound should get closer to the raw positive ratio."
            );

            last_lower_bound = new_lower_bound;
        }
    }
}
