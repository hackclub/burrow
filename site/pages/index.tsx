import { faGithub } from "@fortawesome/free-brands-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import Head from "next/head";
import {
  faChevronDown,
  faChevronUp,
  faUpRightFromSquare,
} from "@fortawesome/free-solid-svg-icons";
import { Menu, Transition } from "@headlessui/react";
import { useState, useRef, useEffect } from "react";
export default function Page() {
  const [chevron, setChevron] = useState(false);
  const menuButtonRef = useRef<HTMLButtonElement | null>(null);
  const toggleDropdown = () => {
    setChevron(!chevron);
  };
  const handleClickOutside = (event: MouseEvent) => {
    if (
      menuButtonRef.current &&
      !menuButtonRef.current.contains(event.target as Node)
    ) {
      setChevron(false);
    }
  };
  useEffect(() => {
    document.addEventListener("click", handleClickOutside);

    return () => {
      document.removeEventListener("click", handleClickOutside);
    };
  }, []);
  return (
    <>
      <Head>
        <title>Burrow</title>
        <meta
          name="description"
          content="Burrow is an open-source tool for bypassing firewalls, built by teenagers at Hack Club."
        />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
      </Head>
      <div className="flex min-h-screen items-center justify-center">
        <div className="mb-48 text-center">
          <h1 className="font-PhantomSans text-5xl md:text-6xl lg:text-7xl xl:text-8xl 2xl:text-9xl">
            <span className="font-bold text-hackClubRed">Burrow</span> Through{" "}
            Firewalls
          </h1>
          <div className="mx-auto my-2 w-11/12 font-PhantomSans text-lg md:my-10 md:w-3/4 lg:text-xl xl:text-2xl 2xl:text-3xl">
            <p>
              Burrow is an open source tool for burrowing through firewalls,
              built by teenagers at{" "}
              <span className="text-hackClubRed underline">
                <a href="https://www.hackclub.com/" target="_blank">
                  Hack Club.
                </a>
              </span>{" "}
              <span className="rounded-md bg-burrowHover p-0.5 font-SpaceMono text-hackClubBlue">
                burrow
              </span>{" "}
              is a Rust-based VPN for getting around restrictive Internet
              censors.
            </p>
          </div>
          <div className="flex flex-wrap justify-center">
            <div className="flex w-full justify-center gap-x-4 md:w-auto">
              <Menu as="div" className="relative inline-block text-left">
                <div>
                  <Menu.Button
                    onClick={() => toggleDropdown()}
                    ref={menuButtonRef}
                    className="w-50 h-12 rounded-2xl bg-hackClubRed px-3 font-SpaceMono hover:scale-105 md:h-12 md:w-auto md:rounded-3xl md:text-xl 2xl:h-16 2xl:text-2xl "
                  >
                    Install for Linux
                    {chevron ? (
                      <FontAwesomeIcon
                        icon={faChevronUp}
                        className="pl-1.5 text-lg"
                      />
                    ) : (
                      <FontAwesomeIcon
                        icon={faChevronDown}
                        className="pl-1.5 text-lg"
                      />
                    )}
                  </Menu.Button>
                </div>
                <Transition
                  enter="transition duration-100 ease-out"
                  enterFrom="transform scale-95 opacity-0"
                  enterTo="transform scale-100 opacity-100"
                  leave="transition duration-75 ease-out"
                  leaveFrom="transform scale-100 opacity-100"
                  leaveTo="transform scale-95 opacity-0"
                >
                  <Menu.Items
                    as="div"
                    className="absolute right-0 z-10 mt-2 h-auto w-auto origin-top-right  divide-black rounded-md bg-hackClubBlueShade font-SpaceMono text-white shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none md:text-xl 2xl:text-2xl"
                  >
                    <div className="divide-y-2 divide-hackClubRed px-1 py-1">
                      <Menu.Item>
                        {({ active }) => (
                          <a className="block px-4 py-2 hover:rounded-lg hover:bg-slate-600">
                            Install for Windows
                          </a>
                        )}
                      </Menu.Item>
                      <Menu.Item>
                        <a className="block px-4 py-2 hover:rounded-lg hover:bg-slate-600">
                          Install for MacOS
                        </a>
                      </Menu.Item>
                    </div>
                  </Menu.Items>
                </Transition>
              </Menu>
              <a>
                <button className="h-12 rounded-2xl border-2 border-hackClubRed bg-transparent px-3 font-SpaceMono text-lg text-hackClubRed hover:scale-110 md:h-12 md:rounded-3xl md:text-xl 2xl:h-16 2xl:text-2xl">
                  Docs
                  <FontAwesomeIcon
                    icon={faUpRightFromSquare}
                    className="pl-3"
                  />
                </button>
              </a>
            </div>
            <div className="mt-4 flex w-full justify-center hover:scale-110 md:mt-0 md:w-auto md:pl-4">
              <a href="https://github.com/hackclub/burrow" target="_blank">
                <button className="h-12 w-40 rounded-xl border-2 border-hackClubRed bg-transparent px-3 font-SpaceMono text-hackClubRed md:h-12 md:w-auto md:rounded-3xl md:text-xl 2xl:h-16 2xl:text-2xl">
                  <FontAwesomeIcon icon={faGithub} className="pr-3" />
                  Contribute
                </button>
              </a>
            </div>
          </div>
          {/* Footer  */}
          {/* <div className="fixed bottom-0 mb-20 left-[25vw] md:left-[40vw] lg:left-[44vw]">
                        <a href="https://hackclub.com/" target="_blank">
                            <button className="flex items-center bg-transparent border-2 border-burrowStroke text-hackClubRed font-SpaceMono text-lg md:text-2xl rounded-xl md:rounded-2xl h-12 md:h-16 px-3">
                                <Image
                                    src="/hackclub.svg"
                                    width={35}
                                    height={35}
                                    className="mx-2"
                                    alt="Hack Club's logo"
                                />
                                By Hack Club
                            </button>
                        </a>
                    </div> */}
        </div>
      </div>
    </>
  );
}
