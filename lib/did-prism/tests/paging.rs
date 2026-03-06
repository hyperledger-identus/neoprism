use identus_did_prism::utils::paging::Paginated;

/// Create a `Paginated` with just the fields that matter for `total_pages()`.
fn paginated(page_size: u32, total_items: u32) -> Paginated<i32> {
    Paginated {
        items: vec![],
        current_page: 0,
        page_size,
        total_items,
    }
}

#[test]
fn total_pages_exact_division() {
    assert_eq!(paginated(10, 30).total_pages(), 3);
}

#[test]
fn total_pages_with_remainder() {
    assert_eq!(paginated(10, 31).total_pages(), 4);
}

#[test]
fn total_pages_zero_items() {
    assert_eq!(paginated(10, 0).total_pages(), 0);
}

#[test]
fn total_pages_one_item() {
    assert_eq!(paginated(10, 1).total_pages(), 1);
}

#[test]
fn total_pages_page_size_one() {
    assert_eq!(paginated(1, 5).total_pages(), 5);
}

#[test]
fn total_pages_single_full_page() {
    assert_eq!(paginated(100, 100).total_pages(), 1);
}

#[test]
fn total_pages_items_less_than_page_size() {
    assert_eq!(paginated(100, 3).total_pages(), 1);
}

#[test]
fn paginated_equality() {
    let p1 = Paginated {
        items: vec![1, 2, 3],
        current_page: 0,
        page_size: 10,
        total_items: 3,
    };
    let p2 = Paginated {
        items: vec![1, 2, 3],
        current_page: 0,
        page_size: 10,
        total_items: 3,
    };
    assert_eq!(p1, p2);
}

#[test]
fn paginated_clone() {
    let p1 = Paginated {
        items: vec![1, 2, 3],
        current_page: 1,
        page_size: 2,
        total_items: 5,
    };
    let p2 = p1.clone();
    assert_eq!(p1, p2);
    assert_eq!(p2.total_pages(), 3);
}
