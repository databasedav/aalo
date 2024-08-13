#[macro_export]
macro_rules! impl_syncers {
    ($struct_name:ident { $($field:ident: $field_ty:ty),* $(,)? }) => {
        paste::paste! {
            impl $struct_name {
                $(
                    pub fn $field(self, $field: $field_ty) -> Self {
                        self.[<$field _signal>](always($field))
                    }

                    pub fn [<$field _signal>](self, [<$field _signal>]: impl Signal<Item = $field_ty> + Send + 'static) -> Self {
                        let syncer = spawn(sync([<$field _signal>], self.$field.clone()));
                        self.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]))
                    }
                )*
            }
        }
    };
}
