use sea_orm::sea_query::SimpleExpr;
use sea_orm::{ColumnTrait, Value};

pub trait ColumnNullTrait<E> {
	fn eq_null(self, option: Option<E>) -> SimpleExpr;
}

impl<T: ColumnTrait, E> ColumnNullTrait<E> for T
where
	Value: From<E>,
{
	fn eq_null(self, option: Option<E>) -> SimpleExpr {
		if let Some(inner) = option {
			self.eq(inner)
		} else {
			self.is_null()
		}
	}
}
