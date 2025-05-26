use sea_orm::sea_query::{Expr, Func, SimpleExpr};
use sea_orm::{ColumnTrait, Value};

pub trait ColumnNullTrait<E> {
	fn eq_null(self, option: Option<E>) -> SimpleExpr;
}

pub trait ColumnEqIgnoreCaseTrait<E> {
	fn eq_ignore_case(self, value: E) -> SimpleExpr;
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

impl<T: ColumnTrait, E> ColumnEqIgnoreCaseTrait<E> for T
where
	Value: From<E>,
{
	fn eq_ignore_case(self, value: E) -> SimpleExpr {
		// LOWER(value) = LOWER($1)
		Expr::expr(Func::lower(Expr::col(self))).eq(Expr::value(value))
	}
}
