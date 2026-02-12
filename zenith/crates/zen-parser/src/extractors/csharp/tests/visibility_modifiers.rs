use super::*;

#[test]
fn maps_csharp_visibility_modifiers() {
    let source = r"
public class Scope {
    public int A;
    internal int B;
    protected int C;
    private protected int D;
}
";

    let items = parse_and_extract(source);

    assert_eq!(find_by_name(&items, "A").visibility, Visibility::Public);
    assert_eq!(
        find_by_name(&items, "B").visibility,
        Visibility::PublicCrate
    );
    assert_eq!(find_by_name(&items, "C").visibility, Visibility::Protected);
    assert_eq!(find_by_name(&items, "D").visibility, Visibility::Protected);
}
