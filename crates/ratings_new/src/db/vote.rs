struct RatingsService {
    db_pool: Vec<()>,
}

impl RatingService {
    // helper methods
}

impl TonicTraitForThing for RatingsService {
    async fn get_vote_summary(&self, snap_name: &str) -> Result<GrpcReplyForVoteSummary, String> {
        let conn = self.get_db_connection().await?;
        let raw = VoteSummary::get_for(snap_name, conn).await?;

        Ok(raw.into())
    }
}
