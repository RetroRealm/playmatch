use crate::sea_orm::ActiveValue::Set;
use entity::signature_group;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// Get the connection and start a transaction
		let db = manager.get_connection();
		let no_intro = signature_group::ActiveModel {
			name: Set("No-Intro".to_string()),
			website_link: Set(Some("https://no-intro.org/".to_string())),
			description: Set(Some("No-Intro catalogs the best available copies of ROMs and digital games, providing DAT files for ROM managers and an online database.".to_string())),
			..Default::default()
		};

		let redump = signature_group::ActiveModel {
			name: Set("Redump".to_string()),
			website_link: Set(Some("http://redump.org/".to_string())),
			description: Set(Some("Redump.org is a disc preservation database and internet community dedicated to collecting precise and accurate information about every video game ever released on optical media of any system. The goal is to make blueprints of the data on console and computer game discs. Redump.org also provides guides to ensure the dumps are correctly done. Users of the website who follow the guides correctly are encouraged to share their results to help build the database. Multiple dumps of games with the same serial number by different people are collected to ensure the same results are gathered, which help correct any incorrect dumps in the database as well as to help recognize alternate versions of the same game.".to_string())),
			..Default::default()
		};

		let tosec = signature_group::ActiveModel {
			name: Set("TOSEC".to_string()),
			website_link: Set(Some("https://www.tosecdev.org/".to_string())),
			description: Set(Some("The Old School Emulation Center (TOSEC) is a retrocomputing initiative dedicated to the cataloging and preservation of software, firmware and resources for arcade machines, microcomputers, minicomputers and video game consoles. The main goal of the project is to catalog and audit various kinds of software and firmware images for these systems.".to_string())),
			..Default::default()
		};

		let mame = signature_group::ActiveModel {
			name: Set("MAME".to_string()),
			website_link: Set(Some("https://mamedev.org/".to_string())),
			description: Set(Some("MAME (formerly an acronym of Multiple Arcade Machine Emulator) is a free and open-source emulator designed to recreate the hardware of arcade game systems in software on modern personal computers and other platforms. Its intention is to preserve gaming history by preventing vintage games from being lost or forgotten. It does this by emulating the inner workings of the emulated arcade machines; the ability to actually play the games is considered \"a nice side effect\". Joystiq has listed MAME as an application that every Windows and Mac gamer should have. The first public MAME release was by Nicola Salmoria on 5 February 1997. It now supports over 7,000 unique games and 10,000 actual ROM image sets, though not all of the games are playable. MESS, an emulator for many video game consoles and computer systems, based on the MAME core, was integrated into MAME in 2015.".to_string())),
			..Default::default()
		};

		signature_group::Entity::insert_many(vec![no_intro, redump, tosec, mame])
			.exec(db)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let db = manager.get_connection();

		signature_group::Entity::delete_many()
			.filter(
				Condition::any()
					.add(signature_group::Column::Name.eq("No-Intro"))
					.add(signature_group::Column::Name.eq("Redump"))
					.add(signature_group::Column::Name.eq("TOSEC"))
					.add(signature_group::Column::Name.eq("MAME")),
			)
			.exec(db)
			.await?;

		Ok(())
	}
}
