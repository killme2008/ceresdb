// Copyright 2022 CeresDB Project Authors. Licensed under Apache-2.0.

//! Implements the TableEngine trait

use std::sync::Arc;

use async_trait::async_trait;
use log::info;
use object_store::ObjectStore;
use snafu::ResultExt;
use table_engine::{
    engine::{
        Close, CloseTableRequest, CreateTableRequest, DropTableRequest, OpenTableRequest, Result,
        TableEngine,
    },
    table::{SchemaId, TableRef},
    ANALYTIC_ENGINE_TYPE,
};
use wal::manager::WalManager;

use crate::{
    context::CommonContext, instance::InstanceRef, meta::Manifest, space::SpaceId,
    sst::factory::Factory, table::TableImpl,
};

/// TableEngine implementation
pub struct TableEngineImpl<Wal, Meta, Store, Fa> {
    /// Instance of the table engine
    instance: InstanceRef<Wal, Meta, Store, Fa>,
}

impl<Wal, Meta, Store, Fa> Clone for TableEngineImpl<Wal, Meta, Store, Fa> {
    fn clone(&self) -> Self {
        Self {
            instance: self.instance.clone(),
        }
    }
}

impl<
        Wal: WalManager + Send + Sync + 'static,
        Meta: Manifest + Send + Sync + 'static,
        Store: ObjectStore,
        Fa,
    > TableEngineImpl<Wal, Meta, Store, Fa>
{
    pub fn new(instance: InstanceRef<Wal, Meta, Store, Fa>) -> Self {
        Self { instance }
    }
}

impl<Wal, Meta, Store, Fa> Drop for TableEngineImpl<Wal, Meta, Store, Fa> {
    fn drop(&mut self) {
        info!("Table engine dropped");
    }
}

#[async_trait]
impl<
        Wal: WalManager + Send + Sync + 'static,
        Meta: Manifest + Send + Sync + 'static,
        Store: ObjectStore,
        Fa: Factory + Send + Sync + 'static,
    > TableEngine for TableEngineImpl<Wal, Meta, Store, Fa>
{
    fn engine_type(&self) -> &str {
        ANALYTIC_ENGINE_TYPE
    }

    async fn close(&self) -> Result<()> {
        info!("Try to close table engine");

        // Close the instance.
        self.instance
            .close()
            .await
            .map_err(|e| Box::new(e) as _)
            .context(Close)?;

        info!("Table engine closed");

        Ok(())
    }

    async fn create_table(&self, request: CreateTableRequest) -> Result<TableRef> {
        let space_id = build_space_id(request.schema_id);

        info!(
            "Table engine impl create table, space_id:{}, request:{:?}",
            space_id, request
        );

        let ctx = CommonContext {
            db_write_buffer_size: self.instance.db_write_buffer_size,
            space_write_buffer_size: self.instance.space_write_buffer_size,
        };
        let space_table = self.instance.create_table(&ctx, space_id, request).await?;

        let table_impl = Arc::new(TableImpl::new(
            space_table,
            self.instance.clone(),
            ANALYTIC_ENGINE_TYPE.to_string(),
        ));

        Ok(table_impl)
    }

    async fn drop_table(&self, request: DropTableRequest) -> Result<bool> {
        let space_id = build_space_id(request.schema_id);

        info!(
            "Table engine impl drop table, space_id:{}, request:{:?}",
            space_id, request
        );

        let ctx = CommonContext {
            db_write_buffer_size: self.instance.db_write_buffer_size,
            space_write_buffer_size: self.instance.space_write_buffer_size,
        };
        let dropped = self.instance.drop_table(&ctx, space_id, request).await?;
        Ok(dropped)
    }

    async fn open_table(&self, request: OpenTableRequest) -> Result<Option<TableRef>> {
        let space_id = build_space_id(request.schema_id);

        info!(
            "Table engine impl open table, space_id:{}, request:{:?}",
            space_id, request
        );
        let ctx = CommonContext {
            db_write_buffer_size: self.instance.db_write_buffer_size,
            space_write_buffer_size: self.instance.space_write_buffer_size,
        };
        let space_table = match self.instance.open_table(&ctx, space_id, &request).await? {
            Some(v) => v,
            None => return Ok(None),
        };

        let table_impl = Arc::new(TableImpl::new(
            space_table,
            self.instance.clone(),
            ANALYTIC_ENGINE_TYPE.to_string(),
        ));

        Ok(Some(table_impl))
    }

    async fn close_table(&self, request: CloseTableRequest) -> Result<()> {
        let space_id = build_space_id(request.schema_id);

        info!(
            "Table engine impl close table, space_id:{}, request:{:?}",
            space_id, request,
        );

        let ctx = CommonContext {
            db_write_buffer_size: self.instance.db_write_buffer_size,
            space_write_buffer_size: self.instance.space_write_buffer_size,
        };
        self.instance.close_table(&ctx, space_id, request).await?;

        Ok(())
    }
}

/// Generate the space id from the schema id with assumption schema id is unique
/// globally.
#[inline]
fn build_space_id(schema_id: SchemaId) -> SpaceId {
    schema_id.as_u32()
}
