use sqlx::FromRow;

use crate::features::pb::common as pb;

const INSUFFICIENT_VOTES_QUANTITY: usize = 25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RatingsBand {
    VeryGood = 0,
    Good = 1,
    Neutral = 2,
    Poor = 3,
    VeryPoor = 4,
    InsufficientVotes = 5,
}

impl RatingsBand {
    const GOOD_UPPER: f64 = 0.8;
    const NEUTRAL_UPPER: f64 = 0.55;
    const POOR_UPPER: f64 = 0.45;
    const VERY_POOR_UPPER: f64 = 0.2;

    pub fn from_value(value: f64) -> RatingsBand {
        if value > Self::GOOD_UPPER {
            RatingsBand::VeryGood
        } else if value <= Self::GOOD_UPPER && value > Self::NEUTRAL_UPPER {
            RatingsBand::Good
        } else if value <= Self::NEUTRAL_UPPER && value > Self::POOR_UPPER {
            RatingsBand::Neutral
        } else if value <= Self::POOR_UPPER && value > Self::VERY_POOR_UPPER {
            RatingsBand::Poor
        } else {
            RatingsBand::VeryPoor
        }
    }
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

    pub(crate) fn into_dto(self) -> pb::Rating {
        pb::Rating {
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
    if total_ratings < INSUFFICIENT_VOTES_QUANTITY {
        return RatingsBand::InsufficientVotes;
    }
    let positive_ratings = votes
        .into_iter()
        .filter(|vote| vote.vote_up)
        .collect::<Vec<Vote>>()
        .len();
    let adjusted_ratio = confidence_interval_lower_bound(positive_ratings, total_ratings);

    RatingsBand::from_value(adjusted_ratio)
}

fn confidence_interval_lower_bound(positive_ratings: usize, total_ratings: usize) -> f64 {
    if total_ratings == 0 {
        return 0.0;
    }

    // Lower Bound of Wilson Score Confidence Interval for Ranking Snaps
    //
    // Purpose:
    // Provides a conservative adjusted rating for a Snap by offsetting the
    // actual ratio of positive votes. It penalizes Snaps with fewer ratings
    // more heavily to produce an adjusted ranking that approaches the mean as
    // ratings increase.
    //
    // Algorithm:
    // Starts with the observed proportion of positive ratings, adjusts it based on
    // total ratings, and incorporates a 95% confidence interval Z-score for
    // uncertainty.
    //
    // References:
    // - https://www.evanmiller.org/how-not-to-sort-by-average-rating.html
    // - https://en.wikipedia.org/wiki/Binomial_proportion_confidence_interval#Wilson_score_interval

    let z_score: f64 = 1.96; // hardcoded for a ~95% confidence
    let total_ratings = total_ratings as f64;
    let positive_ratings_ratio = positive_ratings as f64 / total_ratings;
    ((positive_ratings_ratio + (z_score * z_score) / (2.0 * total_ratings))
        - z_score
            * f64::sqrt(
                (positive_ratings_ratio * (1.0 - positive_ratings_ratio)
                    + ((z_score * z_score) / (4.0 * total_ratings)))
                    / total_ratings,
            ))
        / (1.0 + (z_score * z_score) / total_ratings)
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

    #[test]
    fn test_insufficient_votes() {
        let votes = vec![Vote { vote_up: true }];
        let band = calculate_band(votes);
        assert_eq!(
            band,
            RatingsBand::InsufficientVotes,
            "Should return InsufficientVotes when not enough votes exist for a given Snap."
        )
    }

    #[test]
    fn test_sufficient_votes() {
        let votes = vec![Vote { vote_up: true }; INSUFFICIENT_VOTES_QUANTITY];
        let band = calculate_band(votes);
        assert_eq!(
            band,
            RatingsBand::VeryGood,
            "Should return very good for a sufficient number of all positive votes."
        )
    }
}
