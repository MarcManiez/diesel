use std::any::{Any, TypeId};
use super::QueryFragment;

/// Uniquely identifies queries by their type for the purpose of prepared
/// statement caching.
///
/// All types which implement `QueryFragment` should also implement this trait
/// (It is not an actual supertrait of `QueryFragment` for boxing purposes).
///
/// This trait is generally implemented by invoking the [`impl_query_id!`]
/// macro.
///
/// See the documentation of [the `QueryId` type] and [`HAS_STATIC_QUERY_ID`]
/// for more details.
///
/// [`impl_query_id!`]: ../macro.impl_query_id.html
/// [the `QueryId` type]: #associatedtype.QueryId
/// [`HAS_STATIC_QUERY_ID`]: #associatedconstant.HAS_STATIC_QUERY_ID
pub trait QueryId {
    /// A type which uniquely represents `Self` in a SQL query.
    ///
    /// Typically this will be a re-construction of `Self` using the `QueryId`
    /// type of each of your type parameters. For example, the type `And<Left,
    /// Right>` would have `type QueryId = And<Left::QueryId, Right::QueryId>`.
    ///
    /// The exception to this is when one of your type parameters does not
    /// affect whether the same prepared statement can be used or not. For
    /// example, a bind parameter is represented as `Bound<SqlType, RustType>`.
    /// The actual Rust type we are serializing does not matter for the purposes
    /// of prepared statement reuse, but a query which has identical SQL but
    /// different types for its bind parameters requires a new prepared
    /// statement. For this reason, `Bound` would have `type QueryId =
    /// Bound<SqlType::QueryId, ()>`.
    ///
    /// If `HAS_STATIC_QUERY_ID` is `false`, you can put any type here
    /// (typically `()`).
    type QueryId: Any;

    /// Can the SQL generated by `Self` be uniquely identified by its type?
    ///
    /// Typically this question can be answered by looking at whether
    /// `unsafe_to_cache_prepared` is called in your implementation of
    /// `QueryFragment::walk_ast`. In Diesel itself, the only type which has
    /// `false` here, but is potentially safe to store in the prepared statement
    /// cache is a boxed query.
    const HAS_STATIC_QUERY_ID: bool = true;

    /// Returns the type id of `Self::QueryId` if `Self::HAS_STATIC_QUERY_ID`.
    /// Returns `None` otherwise.
    ///
    /// You should never need to override this method.
    fn query_id() -> Option<TypeId> {
        if Self::HAS_STATIC_QUERY_ID {
            Some(TypeId::of::<Self::QueryId>())
        } else {
            None
        }
    }
}

impl QueryId for () {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = true;
}

impl<T: QueryId + ?Sized> QueryId for Box<T> {
    type QueryId = T::QueryId;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

impl<'a, T: QueryId + ?Sized> QueryId for &'a T {
    type QueryId = T::QueryId;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

impl<DB> QueryId for QueryFragment<DB> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use prelude::*;
    use super::QueryId;

    table! {
        users {
            id -> Integer,
            name -> VarChar,
        }
    }

    fn query_id<T: QueryId>(_: T) -> Option<TypeId> {
        T::query_id()
    }

    #[test]
    fn queries_with_no_dynamic_elements_have_a_static_id() {
        use self::users::dsl::*;
        assert!(query_id(users).is_some());
        assert!(query_id(users.select(name)).is_some());
        assert!(query_id(users.filter(name.eq("Sean"))).is_some());
    }

    #[test]
    fn queries_with_different_types_have_different_ids() {
        let id1 = query_id(users::table.select(users::name));
        let id2 = query_id(users::table.select(users::id));
        assert_ne!(id1, id2);
    }

    #[test]
    fn bind_params_use_only_sql_type_for_query_id() {
        use self::users::dsl::*;
        let id1 = query_id(users.filter(name.eq("Sean")));
        let id2 = query_id(users.filter(name.eq("Tess".to_string())));

        assert_eq!(id1, id2);
    }

    #[test]
    #[cfg(features = "postgres")]
    fn boxed_queries_do_not_have_static_query_id() {
        use pg::Pg;
        assert!(query_id(users::table.into_boxed::<Pg>()).is_none());
    }
}
