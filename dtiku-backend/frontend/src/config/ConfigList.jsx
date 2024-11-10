import {
  List,
  Datagrid,
  TextField,
  DateField,
  EditButton,
  CreateButton,
} from "react-admin";

function ConfigList() {
  return (
    <List>
      <Datagrid>
        <TextField source="id" />
        <TextField source="key" />
        <TextField source="key_desc" />
        <DateField source="value" />
        <TextField source="created" />
        <TextField source="modified" />
        <EditButton resource="configs" />
      </Datagrid>
    </List>
  );
}

export default ConfigList;
