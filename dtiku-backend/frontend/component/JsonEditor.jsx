import { createJSONEditor } from "vanilla-jsoneditor";
import { useEffect, useRef } from "react";

export default function SvelteJSONEditor(props) {
  const refContainer = useRef(null);
  const refEditor = useRef(null);

  const { style, ...rest } = props;

  useEffect(() => {
    // create editor
    console.log("create editor", refContainer.current);
    refEditor.current = createJSONEditor({
      target: refContainer.current,
      props: {},
    });

    return () => {
      // destroy editor
      if (refEditor.current) {
        console.log("destroy editor");
        refEditor.current.destroy();
        refEditor.current = null;
      }
    };
  }, []);

  // update props
  useEffect(() => {
    if (refEditor.current) {
      console.log("update props", rest);
      refEditor.current.updateProps(rest);
    }
  }, [rest]);

  return (
    <div
      className="vanilla-jsoneditor-react"
      style={style}
      ref={refContainer}
    ></div>
  );
}
