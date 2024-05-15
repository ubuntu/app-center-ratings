//! Common defintions used throughout the ratings service.
use sqlx::FromRow;

use crate::features::pb::common as pb;

/// An arbitrary fixed number of votes we've determined is below the threshold to be meaningful.
const INSUFFICIENT_VOTES_QUANTITY: i64 = 25;

/// A descriptive mapping of a number of ratings to a general indicator of "how good"
/// an app can be said to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(missing_docs)]
pub enum RatingsBand {
    VeryGood = 0,
    Good = 1,
    Neutral = 2,
    Poor = 3,
    VeryPoor = 4,
    #[default]
    InsufficientVotes = 5,
}

impl RatingsBand {
    /// The percentage of votes that denotes an upper bound between good and very good
    const GOOD_UPPER: f64 = 0.8;
    /// The percentage of votes that denotes the line between neutral and good
    const NEUTRAL_UPPER: f64 = 0.55;
    /// The percentage of votes that denotes the line between poor and neutral
    const POOR_UPPER: f64 = 0.45;
    /// The percentage of votes that denotes a line between poor and very poor
    const VERY_POOR_UPPER: f64 = 0.2;

    /// Converts a raw value into a [`RatingsBand`] value by comparing it with the associated
    /// constants of the struct.
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

impl PartialOrd for RatingsBand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if matches!(self, RatingsBand::InsufficientVotes)
            || matches!(other, RatingsBand::InsufficientVotes)
        {
            None
        } else {
            // Negative ratings have a higher value i.e., 0 = Very Good and 4 = Very Poor
            let max = Self::InsufficientVotes as u8;
            (max - (*self as u8)).partial_cmp(&(max - (*other as u8)))
        }
    }
}

impl From<crate::features::pb::common::RatingsBand> for RatingsBand {
    fn from(value: crate::features::pb::common::RatingsBand) -> Self {
        match value {
            pb::RatingsBand::VeryGood => Self::VeryGood,
            pb::RatingsBand::Good => Self::Good,
            pb::RatingsBand::Neutral => Self::Neutral,
            pb::RatingsBand::Poor => Self::Poor,
            pb::RatingsBand::VeryPoor => Self::VeryPoor,
            pb::RatingsBand::InsufficientVotes => Self::InsufficientVotes,
        }
    }
}

/// A descriptive rating object, usually used converted and transferred over the wire.
/// This is an aggregated rating for a snap without holding every raw value, as determined
/// by [`RatingsBand`].
#[derive(Debug, Clone, FromRow, Default)]
pub struct Rating {
    /// The ID of the snap this rating is for
    pub snap_id: String,
    /// The total votes for this snap
    pub total_votes: u64,
    /// The descriptive indicator of "how good" this snap is based
    /// on aggregated ratings.
    pub ratings_band: RatingsBand,
}

impl Rating {
    /// Creates a new [`Rating`] based on a given [`VoteSummary`], by calculating
    /// the required [`RatingsBand`].
    pub fn new(votes: VoteSummary) -> Self {
        let (_, ratings_band) = calculate_band(&votes);
        Self {
            snap_id: votes.snap_id,
            total_votes: votes.total_votes as u64,
            ratings_band,
        }
    }

    /// Converts this into its protobuf version for wire transfer.
    pub(crate) fn into_protobuf_rating(self) -> pb::Rating {
        pb::Rating {
            snap_id: self.snap_id,
            total_votes: self.total_votes,
            ratings_band: self.ratings_band as i32,
        }
    }
}

impl From<crate::features::pb::common::Rating> for Rating {
    fn from(value: crate::features::pb::common::Rating) -> Self {
        Self {
            snap_id: value.snap_id,
            total_votes: value.total_votes,
            ratings_band: crate::features::pb::common::RatingsBand::try_from(value.ratings_band)
                .unwrap()
                .into(),
        }
    }
}

/// A summary of votes for a given snap, this is then aggregated before transfer.
#[derive(Debug, Clone, FromRow)]
pub struct VoteSummary {
    /// The ID of the snap being checked.
    pub snap_id: String,
    /// The total votes this snap has received.
    pub total_votes: i64,
    /// The number of the votes which are positive.
    pub positive_votes: i64,
}

/// Converts a given [`VoteSummary`] into a [`RatingsBand`], if applicable, along with a
/// confidence interval if applicable.
pub fn calculate_band(votes: &VoteSummary) -> (Option<f64>, RatingsBand) {
    if votes.total_votes < INSUFFICIENT_VOTES_QUANTITY {
        return (None, RatingsBand::InsufficientVotes);
    }
    let adjusted_ratio = confidence_interval_lower_bound(votes.positive_votes, votes.total_votes);

    (
        Some(adjusted_ratio),
        RatingsBand::from_value(adjusted_ratio),
    )
}

/// Calculates the Lower Bound of Wilson Score Confidence Interval for Ranking Snaps
///
/// Purpose:
/// Provides a conservative adjusted rating for a Snap by offsetting the
/// actual ratio of positive votes. It penalizes Snaps with fewer ratings
/// more heavily to produce an adjusted ranking that approaches the mean as
/// ratings increase.
///
/// Algorithm:
/// Starts with the observed proportion of positive ratings, adjusts it based on
/// total ratings, and incorporates a 95% confidence interval Z-score for
/// uncertainty.
///
/// References:
/// - https://www.evanmiller.org/how-not-to-sort-by-average-rating.html
/// - https://en.wikipedia.org/wiki/Binomial_proportion_confidence_interval#Wilson_score_interval
fn confidence_interval_lower_bound(positive_ratings: i64, total_ratings: i64) -> f64 {
    if total_ratings == 0 {
        return 0.0;
    }

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
            let positive_ratings = (total_ratings as f64 * ratio).round() as i64;
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
        let votes = VoteSummary {
            snap_id: 1.to_string(),
            total_votes: 1,
            positive_votes: 1,
        };
        let (rating, band) = calculate_band(&votes);
        assert_eq!(
            band,
            RatingsBand::InsufficientVotes,
            "Should return InsufficientVotes when not enough votes exist for a given Snap."
        );
        assert!(
            rating.is_none(),
            "Should return band = None for insufficient votes."
        )
    }

    #[test]
    fn test_sufficient_votes() {
        let votes = VoteSummary {
            snap_id: 1.to_string(),
            total_votes: 100,
            positive_votes: 100,
        };
        let (rating, band) = calculate_band(&votes);
        assert_eq!(
            band,
            RatingsBand::VeryGood,
            "Should return very good for a sufficient number of all positive votes."
        );
        assert!(
            rating > Some(0.7),
            "Should return fairly positive raw rating for this ration and volume of positive votes."
        )
    }
}
