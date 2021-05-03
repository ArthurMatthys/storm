use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    spanned::Spanned, Error, FnArg, Ident, Item, ItemFn, LitStr, PathArguments, PathSegment,
    ReturnType, Type,
};

pub(crate) fn indexing(item: Item) -> TokenStream {
    match &item {
        Item::Fn(f) => indexing_fn(f),
        _ => return Error::new(item.span(), "Only function is supported.").to_compile_error(),
    }
}

fn indexing_fn(f: &ItemFn) -> TokenStream {
    let vis = &f.vis;
    let name = &f.sig.ident;
    let name_str = &name.to_string().to_pascal_case();
    let index_name = Ident::new(&name_str, name.span());
    let index_name_lit = LitStr::new(&name_str, name.span());

    let screaming_snake = name.to_string().to_screaming_snake_case();
    let static_var = Ident::new(&format!("{}_VAR", screaming_snake), name.span());

    let ty = match &f.sig.output {
        ReturnType::Type(_, t) => t,
        ReturnType::Default => {
            return Error::new(f.sig.output.span(), "Index must have a return value.")
                .to_compile_error()
        }
    };

    let mut args = Vec::new();

    for arg in &f.sig.inputs {
        match arg {
            FnArg::Typed(t) => args.push(t),
            FnArg::Receiver(r) => {
                return Error::new(r.span(), "Self is not expected here.").to_compile_error()
            }
        };
    }

    let mut as_ref_decl = Vec::new();
    let mut as_ref_decl_async = Vec::new();
    let mut as_ref_args = Vec::new();
    let mut as_ref_tag = Vec::new();
    let mut as_ref_async_wheres = quote!();
    let mut as_ref_wheres = quote!();
    let mut deps = Vec::new();

    for (index, arg) in args.iter().enumerate() {
        let ty = unref(&arg.ty);
        let ident = Ident::new(&format!("var{}", index), arg.span());

        if is_storm_ctx(ty) {
            as_ref_decl.push(quote!(let #ident = self.ctx;));
            as_ref_decl_async.push(quote!(let #ident = self;));
            as_ref_args.push(ident);
        } else {
            as_ref_decl.push(quote!(let #ident = self.as_ref();));
            as_ref_decl_async
                .push(quote!(let #ident = storm::AsRefAsync::as_ref_async(self).await?;));
            as_ref_tag.push(quote!(storm::Tag::tag(#ident)));
            as_ref_args.push(ident);
            deps.push(quote!(<#ty as storm::Accessor>::register_deps(<#index_name as storm::Accessor>::clear);));

            let ref_async = quote!(storm::AsRefAsync<#ty>);
            let ref_sync = quote!(AsRef<#ty>);

            if as_ref_wheres.is_empty() {
                as_ref_async_wheres = quote!(where Self: #ref_async);
                as_ref_wheres = quote!(where Self: #ref_sync);
            } else {
                as_ref_async_wheres = quote!(#as_ref_async_wheres + #ref_async);
                as_ref_wheres = quote!(#as_ref_wheres + #ref_sync);
            };
        }
    }

    let as_ref_args = quote!(#(#as_ref_args,)*);
    let as_ref_decl = quote!(#(#as_ref_decl)*);
    let as_ref_decl_async = quote!(#(#as_ref_decl_async)*);
    let as_ref_tag = quote!(storm::version_tag::combine(&[#(#as_ref_tag,)*]));
    let get_or_init = quote!({
        let _ = tracing::span!(tracing::Level::DEBUG, #index_name_lit, obj = "index", ev = "create").entered();
        #index_name(#name(#as_ref_args), #as_ref_tag)
    });
    let deps = quote!(#(#deps)*);

    quote! {
        #[static_init::dynamic]
        static #static_var: (storm::TblVar<#index_name>, storm::Deps) = {
            #deps
            Default::default()
        };

        #vis struct #index_name(#ty, storm::VersionTag);

        impl storm::Accessor for #index_name {
            #[inline]
            fn var() -> &'static storm::TblVar<Self> {
                &#static_var.0
            }

            #[inline]
            fn deps() -> &'static storm::Deps {
                &#static_var.1
            }
        }

        impl std::ops::Deref for #index_name {
            type Target = #ty;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<'a, L> AsRef<#index_name> for storm::CtxLocks<'a, L> #as_ref_wheres {
            fn as_ref(&self) -> &#index_name {
                #as_ref_decl
                self.ctx.vars().get_or_init(<#index_name as storm::Accessor>::var(), move || #get_or_init)
            }
        }

        impl storm::AsRefAsync<#index_name> for storm::Ctx #as_ref_async_wheres {
            fn as_ref_async(&self) -> storm::BoxFuture<'_, storm::Result<&'_ #index_name>> {
                let var = <#index_name as storm::Accessor>::var();

                Box::pin(async move {
                    let ctx = self.vars();

                    if let Some(v) = ctx.get(var) {
                        return Ok(v);
                    }

                    #as_ref_decl_async
                    Ok(ctx.get_or_init(var, || #get_or_init))
                })
            }
        }

        impl storm::Tag for #index_name {
            #[inline]
            fn tag(&self) -> storm::VersionTag {
                self.1
            }
        }

        #f
    }
}

fn unref(t: &Type) -> &Type {
    match t {
        Type::Reference(r) => unref(&*r.elem),
        _ => t,
    }
}

fn is_storm_ctx(t: &Type) -> bool {
    match t {
        Type::Path(p) => check_path_segment(&p.path.segments, &["storm", "Ctx"]),
        _ => false,
    }
}

fn check_path_segment<'a, SEGS>(segments: SEGS, idents: &[&str]) -> bool
where
    SEGS: IntoIterator<Item = &'a PathSegment>,
    SEGS::IntoIter: DoubleEndedIterator,
{
    segments
        .into_iter()
        .rev()
        .zip(idents.into_iter().rev())
        .all(|(a, b)| a.ident == b && a.arguments == PathArguments::None)
}
