use identus_did_prism::utils::paging::Paginated;

#[test]
fn total_pages_exact_division() {
    let p = Paginated::<i32> {
        items: vec![],
        current_page: 0,
        page_size: 10,
        total_items: 30,
    };
    assert_eq!(p.total_pages(), 3);
}

#[test]
fn total_pages_with_remainder() {
    let p = Paginated::<i32> {
        items: vec![],
        current_page: 0,
        page_size: 10,
        total_items: 31,
    };
    assert_eq!(p.total_pages(), 4);
}

#[test]
fn total_pages_zero_items() {
    let p = Paginated::<i32> {
        items: vec![],
        current_page: 0,
        page_size: 10,
        total_items: 0,
    };
    assert_eq!(p.total_pages(), 0);
}

#[test]
fn total_pages_one_item() {
    let p = Paginated::<i32> {
        items: vec![1],
        current_page: 0,
        page_size: 10,
        total_items: 1,
    };
    assert_eq!(p.total_pages(), 1);
}

#[test]
fn total_pages_page_size_one() {
    let p = Paginated::<i32> {
        items: vec![],
        current_page: 0,
        page_size: 1,
        total_items: 5,
    };
    assert_eq!(p.total_pages(), 5);
}

#[test]
fn total_pages_single_full_page() {
    let p = Paginated::<i32> {
        items: vec![],
        current_page: 0,
        page_size: 100,
        total_items: 100,
    };
    assert_eq!(p.total_pages(), 1);
}

#[test]
fn total_pages_items_less_than_page_size() {
    let p = Paginated::<i32> {
        items: vec![],
        current_page: 0,
        page_size: 100,
        total_items: 3,
    };
    assert_eq!(p.total_pages(), 1);
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
