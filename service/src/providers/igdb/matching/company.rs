use crate::db::company::get_unmatched_companies_with_limit;
use crate::providers::MetadataProvider;
use crate::providers::igdb::IgdbClient;
use crate::providers::{
	DEFAULT_CHUNK_SIZE, Target, drive_match_pipeline, write_auto_match_failed,
	write_auto_match_success,
};
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::debug;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_companies_to_igdb(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	drive_match_pipeline(
		"company",
		MetadataProviderEnum::Igdb,
		get_unmatched_companies_with_limit,
		match_company_to_igdb,
		igdb_client,
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await
}

fn match_company_to_igdb(
	company: entity::company::Model,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = igdb_client.redis_conn().clone();
		let search_results = igdb_client.search_company_by_name(&company.name).await?;

		for search_result in search_results {
			if search_result.name.to_lowercase() == company.name.to_lowercase() {
				debug!(
					"Matched Company \"{}\" to IGDB Company ID {} (Direct Match)",
					company.name, search_result.id
				);
				write_auto_match_success(
					"igdb",
					MetadataProviderEnum::Igdb,
					Target::Company(company.id),
					search_result.id.to_string(),
					AutomaticMatchReasonEnum::DirectName,
					Some(search_result.name.clone()),
					None,
					&db_conn,
					&mut redis_conn,
				)
				.await?;

				return Ok(());
			}
		}

		debug!("No direct match found for Company: \"{}\"", &company.name);
		write_auto_match_failed(
			"igdb",
			MetadataProviderEnum::Igdb,
			Target::Company(company.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}
