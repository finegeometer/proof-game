// Note that the actual content of the book is part of `index.html`.

// Generate the type of pages, complete with conversion to and from strings, and functions that take you to the next or previous page.
macro_rules! generate {
    ($( $page: ident ),*) => {
        #[derive(Clone, Copy)]
        pub enum BookPage { $( $page ),* }
        use BookPage::*;

        shift!({None $( (Some($page)) )*}, {$( (Some($page)) )* None});

        impl From<BookPage> for &'static str {
            fn from(p : BookPage) -> &'static str {
                match p {
                    $(
                        $page => stringify!($page),
                    )*
                }
            }
        }

        pub struct NoSuchPage;
        impl<'a> TryFrom<&'a str> for BookPage {
            type Error = NoSuchPage;
            fn try_from(s : &'a str) -> Result<BookPage, NoSuchPage> {
                match s {
                    $(
                        stringify!($page) => Ok($page),
                    )*
                    _ => Err(NoSuchPage),
                }
            }
        }
    };
}

macro_rules! shift {
    ({$($p1: tt)*}, {$($p2: tt)*}) => {
        #[allow(unused_parens)]
        impl BookPage {
            pub fn next(self) -> Option<Self> {
                match Some(self) {
                    $(
                        $p1 => $p2,
                    )*
                }
            }
            pub fn prev(self) -> Option<Self> {
                match Some(self) {
                    $(
                        $p2 => $p1,
                    )*
                }
            }
        }
    };
}

generate!(Conjunction, Disjunction, Implication, Equality);
