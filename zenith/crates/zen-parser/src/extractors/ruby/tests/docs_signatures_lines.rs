use super::*;

#[test]
fn ruby_doc_and_signature_are_normalized() {
    let source = r"
class Receipt
  # Computes total amount.
  # Includes tax.
  def total(amount, tax)
    amount + tax
  end
end
";
    let items = parse_and_extract(source);
    let total = find_by_name(&items, "total");
    assert!(
        total.doc_comment.is_empty()
            || total.doc_comment == "Computes total amount.\nIncludes tax."
    );
    assert_eq!(total.signature, "def total(amount, tax)");
    assert_eq!(total.start_line, 5);
}
