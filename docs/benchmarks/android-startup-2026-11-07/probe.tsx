import { useState } from "react";
import { Button, Icon, Screen, Stack, Text } from "ink";
import { add as Add, alarm as Alarm, arrowBack as ArrowBack, arrowForward as ArrowForward, check as Check, close as Close, deleteIcon as Delete, download as Download, edit as Edit, favorite as Favourite, folder as Folder, home as Home, info as Info, mail as Mail, menu as Menu, moreVert as MoreVert, pause as Pause, playArrow as PlayArrow, refresh as Refresh, search as Search, settings as Settings, share as Share, star as Star, stop as Stop, upload as Upload, volumeUp as VolumeUp, warning as Warning, wifi as Wifi } from "ink/icons";

const icons = [Add, Alarm, ArrowBack, ArrowForward, Check, Close, Delete, Download, Edit, Favourite, Folder, Home, Info, Mail, Menu, MoreVert, Pause, PlayArrow, Refresh, Search, Settings, Share, Star, Stop, Upload, VolumeUp, Warning, Wifi];
export default function Counter() {
  const [count, setCount] = useState(0);
  return <Screen title="Startup probe" centered><Stack gap={8} align="center">
    <Text size={24}>Count: {count}</Text>
    {Array.from({ length: 5 }, (_, row) => <Stack key={row} axis="horizontal" gap={4}>
      {icons.slice(row * 6, row * 6 + 6).map((name) => <Icon key={name} name={name} size={48} />)}
    </Stack>)}
    <Button onPress={() => setCount((value) => value + 1)}>Increase</Button>
  </Stack></Screen>;
}
