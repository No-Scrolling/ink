import image0 from "../assets/album-00.jpg";
import image1 from "../assets/album-01.jpg";
import image2 from "../assets/album-02.jpg";
import image3 from "../assets/album-03.jpg";
import image4 from "../assets/album-04.jpg";
import image5 from "../assets/album-05.jpg";
import image6 from "../assets/album-06.jpg";
import image7 from "../assets/album-07.jpg";
import image8 from "../assets/album-08.jpg";
import image9 from "../assets/album-09.jpg";
import image10 from "../assets/album-10.jpg";
import image11 from "../assets/album-11.jpg";
import image12 from "../assets/album-12.jpg";
import image13 from "../assets/album-13.jpg";
import image14 from "../assets/album-14.jpg";
import image15 from "../assets/album-15.jpg";
import image16 from "../assets/album-16.jpg";
import image17 from "../assets/album-17.jpg";
import image18 from "../assets/album-18.jpg";
import image19 from "../assets/album-19.jpg";
import image20 from "../assets/album-20.jpg";
import image21 from "../assets/album-21.jpg";
import image22 from "../assets/album-22.jpg";
import image23 from "../assets/album-23.jpg";

const images = [image0, image1, image2, image3, image4, image5, image6, image7, image8, image9, image10, image11, image12, image13, image14, image15, image16, image17, image18, image19, image20, image21, image22, image23];

export const items = Array.from({ length: 500 }, (_, index) => ({
  id: index + 1, title: `Album ${String(index + 1).padStart(3, "0")}`,
}));

export const artwork = (id: number, alternate: boolean) => images[(id - 1 + (alternate ? 12 : 0)) % images.length];
