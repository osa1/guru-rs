//! A tree widget for showing expressions/values.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk::prelude::*;

pub struct ExpressionsW {
    store: gtk::TreeStore,
    view: gtk::TreeView,
    scrolled: gtk::ScrolledWindow,
    entry: gtk::Entry,
    // The top widget
    box_: gtk::Box,
    // Shared mutable cell to be able to use in callbacks
    exprs: Rc<RefCell<Vec<ExpressionChild>>>,
    // Callback for getting children of an expression, used when expanding tree nodes.
    get_children: ExprGetChildrenCb,
    // Callback for adding new expressions
    add_expr: AddExprCb,
}

/// Type of the reference for the callback for asking for children of an expression.
type ExprGetChildrenCb = Rc<RefCell<Option<Box<Fn(&str /* full name of the expression */)>>>>;

/// Type of the reference for the callback for adding new expressions.
type AddExprCb = Rc<RefCell<Option<Box<Fn(String /* expression */)>>>>;

// TODO: Rename this
struct ExpressionChild {
    /// Location of this node in the tree.
    iter: gtk::TreeIter,

    /// Full name of the expression. Not rendered, passed to callbacks to update state.
    /// E.g. "x.y.z"
    full_name: String,

    /// Name of the current node in the parent. E.g. "y" when this is the node "x.y". Not rendered.
    name: String,

    /// The expression.
    expr: String,
    value: Option<String>,
    type_: Option<String>,

    /// Children of this node.
    children: Vec<ExpressionChild>,
}

// TODO: Put the tree in a scrolled window

impl ExpressionsW {
    pub fn new() -> ExpressionsW {
        let store = gtk::TreeStore::new(&[
            String::static_type(), // full name (not rendered)
            String::static_type(), // expression
            String::static_type(), // value
            String::static_type(), // type
        ]);

        // Without this we can't store and reuse TreeIters
        assert!(store
            .get_flags()
            .contains(gtk::TreeModelFlags::ITERS_PERSIST));

        // box -> [ scrolled -> tree view, entry ]

        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
        box_.set_baseline_position(gtk::BaselinePosition::Top);

        let scrolled = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
        scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
        box_.pack_start(&scrolled, true, true, 0);

        let view = gtk::TreeView::new_with_model(&store);
        scrolled.add(&view);

        let entry = gtk::Entry::new();
        box_.pack_end(&entry, false, false, 0);

        let exprs = Rc::new(RefCell::new(vec![]));

        //
        // Create columns
        //

        let add_text_col = |name: &str, idx: i32| {
            let cell_renderer = gtk::CellRendererText::new();
            let col = gtk::TreeViewColumn::new();
            col.set_title(name);
            col.pack_start(&cell_renderer, true);
            col.add_attribute(&cell_renderer, "text", idx);
            view.append_column(&col);
        };

        add_text_col("Expression", 1);
        add_text_col("Value", 2);
        add_text_col("Type", 3);

        let add_expr: AddExprCb = Rc::new(RefCell::new(None));

        //
        // Connect row expanded signal
        //

        let tree = exprs.clone();
        let get_children: ExprGetChildrenCb = Rc::new(RefCell::new(None));
        let get_children_clone = get_children.clone();
        view.connect_row_expanded(move |view: &gtk::TreeView, iter, path| {
            let model = view.get_model().unwrap();
            let store = model.downcast_ref::<gtk::TreeStore>().unwrap();
            let name = store.get_value(&iter, 0).get::<String>().unwrap();
            let expr = store.get_value(&iter, 1).get::<String>().unwrap();

            // The TreeStore path of the node should be the same as its path in `ExpressionsW.exprs`
            // TODO somehow check this
            let path = path.get_indices();
            assert!(!path.is_empty());
            let mut node: &ExpressionChild = &tree.borrow()[path[0] as usize];
            for p in &path[1..] {
                node = &node.children[*p as usize];
            }

            // If the node doesn't have any children yet ask for it
            // FIXME: What if it doesn't have children? we keep asking for it every time we expand!
            if node.children.is_empty() {
                match *get_children_clone.borrow() {
                    None => {
                        println!("Can't get children of {}: callback not set", node.full_name);
                    }
                    Some(ref cb) => {
                        cb(&node.full_name);
                    }
                }
            }
        });

        //
        // Connect expression added signal
        //

        let add_expr_clone = add_expr.clone();
        entry.connect_activate(move |w| {
            if let Some(expr_str) = w.get_text() {
                if expr_str.as_str().is_empty() {
                    return;
                }
                let expr_str = expr_str.as_str().to_string();

                match *add_expr_clone.borrow() {
                    None => {
                        println!(
                            "\"Expression added\" callback is not set. Ignoring expression: {:?}",
                            expr_str
                        );
                    }
                    Some(ref cb) => {
                        cb(expr_str);
                    }
                }

                w.set_text("");
            }
        });

        ExpressionsW {
            store,
            view,
            scrolled,
            entry,
            box_,
            exprs,
            get_children,
            add_expr,
        }
    }

    pub fn get_widget(&self) -> &gtk::Widget {
        self.box_.upcast_ref()
    }

    /// Set "get children" callback. Argument to the callback is the full name of the expression,
    /// e.g. "var1.x.y".
    pub fn connect_get_children(&mut self, cb: Box<Fn(&str)>) {
        *self.get_children.borrow_mut() = Some(cb);
    }

    pub fn connect_add_expr(&mut self, cb: Box<Fn(String)>) {
        *self.add_expr.borrow_mut() = Some(cb);
    }

    /// Add a top-level expression.
    fn add_top(
        &mut self,
        full_name: String,
        expr: String,
        value: String,
        type_: String,
        has_children: bool,
    ) {
        // Insert a row
        let iter = self.store.insert(None /* parent */, -1 /* last */);
        self.store.set(
            &iter,
            &[0, 1, 2, 3],
            &[
                &full_name.to_value(),
                &expr.to_value(),
                &value.to_value(),
                &type_.to_value(),
            ],
        );

        // Create a placeholder iter for children if this has children.
        if has_children {
            let iter = self.store.insert(&iter, -1);
            self.store
                .set(&iter, &[0, 1], &[&"__PLACEHOLDER__", &"__PLACEHOLDER__"]);
        }

        let node = ExpressionChild {
            iter,
            full_name: full_name.clone(),
            name: full_name,
            expr,
            value: Some(value),
            type_: Some(type_),
            children: vec![],
        };

        self.exprs.borrow_mut().push(node);
    }

    pub fn add(
        &mut self,
        name: String,
        expr: String,
        value: String,
        type_: String,
        has_children: bool,
    ) {
        let path = name.split('.').collect::<Vec<_>>();
        if path.len() == 1 {
            // Add a top-level expression
            self.add_top(name, expr, value, type_, has_children);
        } else {
            // Otherwise start recursing down to find/create the node we're looking for
            let top_level_name = path[0];
            for node in self.exprs.borrow_mut().iter_mut() {
                if node.name == top_level_name {
                    return add_child(
                        &self.store,
                        node,
                        &path[1..],
                        name.clone(), // sigh
                        expr,
                        value,
                        type_,
                        has_children,
                    );
                }
            }
        }
    }
}

/// Add a child expression.
fn add_child(
    store: &gtk::TreeStore,
    // Searching for the parent in this node.
    mut node: &mut ExpressionChild,
    // Path of the child node. When this has one entry `node` is the parent of the child. Can't be
    // empty.
    path: &[&str],     // ["y", "z"]
    full_name: String, // "x.y.z"
    expr: String,
    value: String,
    type_: String,
    has_children: bool,
) {
    // Find index of this node in the parent (`node`)
    let mut node_idx: Option<usize> = None;
    let p = path[0];
    for i in 0..node.children.len() {
        if node.children[i].name == p {
            node_idx = Some(i);
            break;
        }
    }

    match node_idx {
        None => {
            // We don't have a node for the child yet, create it.
            // We don't support adding deep nodes, so if we reached to this case the parent for
            // this node needs to exist.
            assert!(path.len() == 1);

            // We should have at least one "placeholder" in as a child in this node, so this should
            // work
            let mut store_path = store.get_path(&node.iter).unwrap();
            store_path.down();

            let insert = |iter| {
                store.set(
                    iter,
                    &[0, 1, 2, 3],
                    &[
                        &full_name.to_value(),
                        &expr.to_value(),
                        &value.to_value(),
                        &type_.to_value(),
                    ],
                );
            };

            // Update the placeholder if it exists
            let mut iter = store.get_iter(&store_path).unwrap();
            let iter_name = store.get_value(&iter, 0).get::<String>().unwrap();
            if iter_name.as_str() == "__PLACEHOLDER__" {
                insert(&iter);
            } else {
                // Otherwise insert new row to the end
                iter = store.insert(&node.iter, -1);
                insert(&iter);
            }

            // Create a placeholder iter for children if the new node has children
            if has_children {
                let iter = store.insert(&iter, -1);
                store.set(&iter, &[0, 1], &[&"__PLACEHOLDER__", &"__PLACEHOLDER__"]);
            }

            // Insert a new node in the model
            node.children.push(ExpressionChild {
                iter,
                full_name,
                name: path[0].to_owned(),
                expr,
                value: Some(value),
                type_: Some(type_),
                children: vec![],
            });
        }
        Some(node_idx) => {
            if path.len() == 1 {
                // Update the store
                store.set(
                    &node.children[node_idx].iter,
                    &[0, 1, 2, 3],
                    &[
                        &full_name.to_value(),
                        &expr.to_value(),
                        &value.to_value(),
                        &type_.to_value(),
                    ],
                );
                // Update the node at node_idx
                let mut node = &mut node.children[node_idx];
                node.full_name = full_name;
                node.name = path[0].to_owned();
                node.expr = expr;
                node.value = Some(value);
                node.type_ = Some(type_);
            } else {
                add_child(
                    store,
                    &mut node.children[node_idx],
                    &path[1..],
                    full_name,
                    expr,
                    value,
                    type_,
                    has_children,
                )
            }
        }
    }
}

//
// Refreshing the state after gdb runs
//

impl ExpressionsW {
    pub fn refresh(&self) {
        // TODO: Should I be using `-var-update --all-values *` ?
        // TODO: This doesn't handle top-level expressions with no children
        let get_children_borrow = self.get_children.borrow();
        let get_children: &Fn(&str) = &**get_children_borrow.as_ref().unwrap(); // welp
        for expr in self.exprs.borrow().iter() {
            expr.refresh(get_children);
        }
    }
}

impl ExpressionChild {
    pub fn refresh(&self, get_children: &Fn(&str)) {
        get_children(&self.name);
    }
}
